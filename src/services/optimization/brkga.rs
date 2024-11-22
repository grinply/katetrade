use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use itertools::Itertools;
use rayon::prelude::*;
use rand::prelude::*;
use rand_pcg::{Pcg64, Lcg128Xsl64};
use simple_bar::ProgressBar;
use crate::services::configuration::kate_configuration_directory;

/// BRKGA: Biased Random Key Genetic Algorithm.
///
/// BRKGA is an evolutionary algorithm used for optimization. In the current trading context
/// it can be used to find parameters hat maximize a given performance metric such as total return.
///
/// In order to use the optimizer you must specify how to evaluate each solution ( calculate its fitness ).
/// implementing the trait [FitnessExecutor], for a concrete example refer to
/// [crate::strategies::custom::trade_rule_strategy::optimization::BacktestOptimizerTradeRule]
pub fn run_brkga_optimization(
    seed: u64,
    config: BrkgaConfig,
    fitness_executor: Box<dyn FitnessExecutor>,
    save_results_to_disk: bool,
) -> Vec<Individual> {
    let mut brkga_evolutionary_optimizer = BRKGA::new(
        seed,
        config,
        fitness_executor,
    );
    brkga_evolutionary_optimizer.run(save_results_to_disk);
    brkga_evolutionary_optimizer.population
}

/// trait that must be implemented for anything that you want to be optimized using BRKGA
/// required in order to run [run_brkga_optimization]
pub trait FitnessExecutor: Send + Sync {
    fn calculate_fitness(&self, chromosome: &[f32]) -> f64;
    fn chromosome_details(&self, chromosome: &[f32]) -> String;
    fn is_chromosome_valid(&self, chromosome: &[f32]) -> bool;
    fn chromosome_size(&self) -> usize;
}

/// configuration parameters used when running optimizations with BRKGA
/// with the function [run_brkga_optimization]
pub struct BrkgaConfig {
    fraction_top: f32,
    fraction_bottom: f32,
    population_size: usize,
    max_iterations: usize,
    elitism_rate: f32,
}

struct BRKGA {
    fraction_top: f32, // amount of individuals considered elite
    fraction_bottom: f32, // amount of mutants to be introduced in each generation
    population_size: usize, // size of the population
    max_iterations: usize,
    elitism_rate: f32, // percentage chance from a gene to be selected from the elite parent
    rng: Lcg128Xsl64,
    chromosome_size: usize,
    population: Vec<Individual>,
    fitness_executor: Box<dyn FitnessExecutor>,
}

impl BrkgaConfig {
    pub fn new(
        fraction_top: f32,
        fraction_bottom: f32,
        population_size: usize,
        max_iterations: usize,
        elitism_rate: f32
    ) -> Self {
        BrkgaConfig {
            fraction_top,
            fraction_bottom,
            population_size,
            max_iterations,
            elitism_rate
        }
    }
}

impl BRKGA {
    fn new(
        seed: u64,
        config: BrkgaConfig,
        fitness_executor: Box<dyn FitnessExecutor>,
    ) -> Box<Self> {
        Box::new(
            BRKGA {
                fraction_top: config.fraction_top,
                fraction_bottom: config.fraction_bottom,
                population_size: config.population_size,
                max_iterations: config.max_iterations,
                elitism_rate: config.elitism_rate,
                chromosome_size: fitness_executor.chromosome_size(),
                fitness_executor,
                rng: Pcg64::seed_from_u64(seed),
                population: vec![],
            }
        )
    }

    fn initial_population(&mut self) -> Vec<Individual> {
        (0..self.population_size).map(|_| self.random_individual()).collect()
    }

    pub fn random_individual(&mut self) -> Individual {
        loop {
            let chromosome = (0..self.chromosome_size).map(|_| self.rng.gen_range(0.0..=1.0)).collect_vec();
            let value = chromosome.as_slice();

            if self.fitness_executor.is_chromosome_valid(value) {
                return Individual::new(chromosome);
            }
        }
    }

    fn generate_mutants(&mut self, fraction_of_population: f32) -> Vec<Individual> {
        let amount_mutants = (self.population_size as f32 * fraction_of_population) as usize;
        (0..amount_mutants).map(|_| self.random_individual()).collect()
    }

    fn get_elite_population(&self, fraction_of_population: f32) -> Vec<Individual> {
        let amount_elite = (self.population_size as f32 * fraction_of_population) as usize;
        let elite_start = self.population_size - amount_elite;
        self.population[elite_start..].to_vec()
    }

    fn sort_population(&mut self) {
        self.population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness)
            .expect("invalid fitness value, make sure all fitness value are number and not 'nan' or 'inf'"));
    }

    fn calculate_population_fitness(&mut self) {
        let mut individuals_without_fitness_calculated: Vec<&mut Individual> = self.population
            .iter_mut()
            .filter(|ind| ind.fitness.is_none())
            .collect();

        let fitness_bar = Arc::new(Mutex::new(ProgressBar::default(individuals_without_fitness_calculated.len() as u32, 100, false)));

        println!("calculating fitness for {} individuals", individuals_without_fitness_calculated.len());

        individuals_without_fitness_calculated
            .par_iter_mut()
            .for_each(|ind| {
                let bar = Arc::clone(&fitness_bar);
                let chromossome = ind.chromosome.as_slice();
                ind.fitness = Some(self.fitness_executor.calculate_fitness(chromossome) as f32);
                let mut parse_progress_bar = bar.lock().unwrap();
                parse_progress_bar.update();
            });
    }

    fn crossover(&mut self, index_elite_parent: usize, index_non_elite_parent: usize) -> Individual {
        let mut child = Individual::new(Vec::with_capacity(self.chromosome_size));


        for i in 0..self.chromosome_size {
            if self.rng.gen_range(0.0..1.0) < self.elitism_rate {
                child.chromosome.push(self.population[index_elite_parent].chromosome[i]);
            } else {
                child.chromosome.push(self.population[index_non_elite_parent].chromosome[i]);
            }
        }
        child
    }

    fn evolve_population(&mut self) -> Vec<Individual> {
        let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);

        let best_fitness = self.population.last().unwrap().fitness.unwrap();
        let worse_fitness = self.population.first().unwrap().fitness.unwrap();

        if best_fitness > worse_fitness {
            new_population.extend(self.generate_mutants(self.fraction_bottom));
            new_population.extend(self.get_elite_population(self.fraction_top));
        } else {
            new_population.extend(self.generate_mutants(1.0));
        }

        let amount_to_reproduce = self.population_size - new_population.len();
        for _ in 0..amount_to_reproduce {
            let index_parent_1 = self.get_random_elite();
            let index_parent_2 = self.get_random_non_elite();
            let child = self.crossover(index_parent_1, index_parent_2);
            new_population.push(child);
        }

        new_population
    }

    // returns a random elite individual from the population
    fn get_random_elite(&mut self) -> usize {
        let amount_elite = (self.population_size as f32 * self.fraction_top) as usize;
        let elite_start = self.population_size - amount_elite;
        self.rng.gen_range(elite_start..self.population_size)
    }

    // returns the index of a random not elite individual from the population
    fn get_random_non_elite(&mut self) -> usize {
        let amount_elite = (self.population_size as f32 * self.fraction_top) as usize;
        let elite_start = self.population_size - amount_elite;
        self.rng.gen_range(0..elite_start)
    }
    // save_results_on_folder: Option<PathBuf>)
    fn run(&mut self, save_results: bool) {
        println!("starting optimization with a population of {} individuals", self.population_size);
        let start = Instant::now();
        self.population = self.initial_population();

        for i in 0..self.max_iterations {
            println!("\niteration #{}", i);
            let start_iteration = Instant::now();

            self.calculate_population_fitness();
            self.sort_population();
            self.show_details(i);
            if save_results {
                self.save_results_to_disk(i);
            }
            self.population = self.evolve_population();

            let duration_iteration = start_iteration.elapsed();
            println!("time elapsed is: {:.2?}", duration_iteration);
        }

        let duration = start.elapsed();
        println!("Time elapsed is: {:?}", duration);
    }

    fn save_results_to_disk(&self, generation: usize) {
        let experiment_directory = kate_configuration_directory()
            .join("experiments");

        if !experiment_directory.exists() {
            fs::create_dir_all(&experiment_directory).expect("should be allowed to create experiment directory");
        }

        let file_name = format!("generation_{generation}__{:?}", chrono::offset::Utc::now());

        let mut experiment_file = File::create(experiment_directory.join(file_name).as_path())
            .expect("should be allowed to open experiment file");

        experiment_file.write_fmt(format_args!("best fitness: {}\n", self.population.last().unwrap().fitness.unwrap())).unwrap();
        experiment_file.write_fmt(format_args!("{}\n\n", self.fitness_executor
            .chromosome_details(self.population.last().unwrap().chromosome.as_slice()))).unwrap();

        experiment_file.write_fmt(format_args!("#solutions:\n")).unwrap();
        for individual in &self.population {
            let value = format!("{:?}", individual.chromosome.as_slice());
            experiment_file.write_fmt(format_args!("{}\n", value)).unwrap();
        }

        experiment_file.flush().unwrap();
        experiment_file.sync_all().unwrap();
        println!("chromosomes from generation {} file saved on disk", generation);
    }

    fn show_details(&self, _: usize) {
        let best = self.population.last().unwrap();
        let median = &self.population[self.population_size / 2];
        let worst = self.population.first().unwrap();
        println!("best fitness: {}", best.fitness.unwrap());
        println!("median fitness: {}", median.fitness.unwrap());
        println!("worst fitness: {}", worst.fitness.unwrap());
        println!("\nbest chromosome {:?}", best.chromosome);
        println!("{}", self.fitness_executor.chromosome_details(best.chromosome.as_slice()));
    }
}

pub struct Individual {
    pub fitness: Option<f32>,
    pub chromosome: Vec<f32>,
}

impl Individual {
    pub fn new(chromosome: Vec<f32>) -> Self {
        Self {
            fitness: None,
            chromosome,
        }
    }
}

impl Clone for Individual {
    fn clone(&self) -> Individual {
        Individual {
            fitness: self.fitness,
            chromosome: self.chromosome.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_individual() {
        let config = BrkgaConfig::new(0.1, 0.2, 15000, 1000, 0.6);
        let mut brkga = BRKGA::new(59841, config, Box::new(TestFitnessExecutor {}));
        let first_indivual = brkga.random_individual();

        assert_eq!(first_indivual.chromosome.len(), 10);

        for i in 0..first_indivual.chromosome.len() {
            assert!(first_indivual.chromosome[i] >= 0.0 && first_indivual.chromosome[i] <= 1.0);
        }

        assert_eq!(0.6609407, first_indivual.chromosome[0]);
        assert_eq!(0.3866915, first_indivual.chromosome[9]);

        let second_individual = brkga.random_individual();

        assert_ne!(first_indivual.chromosome, second_individual.chromosome);
    }

    struct TestFitnessExecutor {}

    impl FitnessExecutor for TestFitnessExecutor {
        fn calculate_fitness(&self, chromosome: &[f32]) -> f64 {
            let mut sum = 0;
            for i in 0..chromosome.len() {
                if chromosome[i] > 0.5 {
                    sum += 1;
                }
            }

            sum as f64
        }

        fn chromosome_details(&self, _: &[f32]) -> String {
            "".to_string()
        }

        fn is_chromosome_valid(&self, _: &[f32]) -> bool {
            true
        }

        fn chromosome_size(&self) -> usize {
            10
        }
    }
}


