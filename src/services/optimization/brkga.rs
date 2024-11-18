use std::sync::{Arc, Mutex};
use std::time::Instant;
use rayon::prelude::*;
use rand::prelude::*;
use rand_pcg::{Pcg64, Lcg128Xsl64};
use simple_bar::ProgressBar;
use crate::services::backtest::{BacktestMetrics, TradingBacktester};
use crate::strategies::custom::trade_rule_strategy::TradeRuleStrategy;



pub struct BRKGA {
    fraction_top: f32, // amount of individuals considered elite
    fraction_bottom: f32, // amount of mutants to be introduced in each generation
    population_size: usize, // size of the population
    max_iterations: usize,
    elitism_rate: f32, // percentage chance from a gene to be selected from the elite parent
    rng: Lcg128Xsl64,
    chromosome_size: usize,
    population: Vec<Individual>,
    fitness_executor: FitnessExecutor,
}

trait FitnessFunction {
    fn fitness(&self, chromosome: Vec<f32>) -> f32;
}

pub type BrkgaConfig = (f32, f32, usize, usize, f32);

impl BRKGA {
    pub fn new(seed: u64, chromosome_size: usize, config: BrkgaConfig, fitness_executor: FitnessExecutor) -> Self {
        Self {
            fraction_top: config.0,
            fraction_bottom: config.1,
            population_size: config.2,
            max_iterations: config.3,
            elitism_rate: config.4,
            chromosome_size,
            fitness_executor,
            rng: Pcg64::seed_from_u64(seed),
            population: vec![],
        }
    }

    fn initial_population(&mut self) -> Vec<Individual>{
        (0..self.population_size).map(|_| self.random_individual()).collect()
    }

    pub fn random_individual(&mut self) -> Individual {
        Individual::new((0..self.chromosome_size).map(|_|self.rng.gen_range(0.0..=1.0)).collect())
    }

    fn generate_mutants(&mut self, fraction_of_population: f32) -> Vec<Individual> {
        let amount_mutants = (self.population_size as f32 * fraction_of_population) as usize;
        (0..amount_mutants).map(|_|self.random_individual()).collect()
    }

    fn get_elite_population(&self, fraction_of_population: f32) -> Vec<Individual> {
        let amount_elite = (self.population_size as f32 * fraction_of_population) as usize;
        let elite_start = self.population_size - amount_elite;
        self.population[elite_start..].to_vec()
    }

    fn sort_population(&mut self) {
        self.population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());
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
                ind.fitness = Some(self.fitness_executor.calculate_fitness(ind.chromosome.as_slice()));
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

    fn evolve_population(&mut self) -> Vec<Individual>{
        let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);

        let best_fitness = self.population.last().unwrap().fitness.unwrap();
        let worse_fitness = self.population.first().unwrap().fitness.unwrap();

        if best_fitness > worse_fitness {
            new_population.extend(self.generate_mutants(self.fraction_bottom));
            new_population.extend(self.get_elite_population(self.fraction_top));
        }else{
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

    pub fn run(&mut self) {
        println!("starting optimization with a population of {} individuals", self.population_size);
        let start = Instant::now();
        self.population = self.initial_population();

        for i in 0..self.max_iterations {
            println!("\niteration #{}", i);
            let start_iteration = Instant::now();
            self.calculate_population_fitness();
            self.sort_population();
            self.show_details(i);
            self.population = self.evolve_population();
            let duration_iteration = start_iteration.elapsed();
            println!("time elapsed is: {:.2?}", duration_iteration);
        }

        let duration = start.elapsed();
        println!("Time elapsed is: {:?}", duration);
    }

    fn show_details(&self, generation: usize) {
        let best = &self.population[self.population_size-1];
        let median = &self.population[self.population_size/2];
        let worst = &self.population[0];
        println!("best fitness: {}", best.fitness.unwrap());
        println!("median fitness: {}", median.fitness.unwrap());
        println!("worst fitness: {}", worst.fitness.unwrap());
        println!("best chromosome {:?}", best.chromosome);
        let trading_model = TradeRuleStrategy::decode(best.chromosome.as_slice());
        let metrics = self.fitness_executor.backtester.run_backtest(
            trading_model,
            self.fitness_executor.qty_per_trade,
            self.fitness_executor.fixed_amount_per_trade
        );
        println!("{}", metrics);
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

#[derive()]
pub struct FitnessExecutor {
    qty_per_trade: u32,
    fixed_amount_per_trade: bool,
    backtester: TradingBacktester<'static>,
}

impl FitnessExecutor {
    pub fn new(backtester: TradingBacktester<'static>) -> Self {
        Self {
            backtester,
            qty_per_trade: 10,
            fixed_amount_per_trade: true
        }
    }

    pub fn calculate_fitness(&self, chromosome: &[f32]) -> f32 {
        let trading_model = TradeRuleStrategy::decode(chromosome);
        let metrics = self.backtester.run_backtest(trading_model, self.qty_per_trade, self.fixed_amount_per_trade);
        Self::fitness_from_metrics(metrics) as f32
    }

    fn fitness_from_metrics(metrics: BacktestMetrics) -> f64 {
        // if metrics.avg_losing_trade.is_nan() || metrics.avg_winning_trade.is_nan() || metrics.max_draw_down_percentage.is_nan() {
        //     -10_000.0
        // }else{
        //     (metrics.avg_winning_trade * metrics.total_wins as f64) - (metrics.avg_losing_trade * metrics.total_loses as f64)
        // }
        if metrics.total_trades == 0 {
            -100.0
        }else{
            metrics.total_return_percentage
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::asset::Asset;
    use crate::models::candlestick::CandlestickType;
    use crate::models::exchange::Exchange;
    use crate::models::market::Market;
    use crate::strategies::strategy::CandlestickConfiguration;
    use super::*;

    #[test]
    fn test_new_brkga(){
        let config: BrkgaConfig = (0.1, 0.2, 15000, 1000, 0.6);

        let backtest_engine = TradingBacktester::new(
            Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap(),
            20150102,
            20150102,
            CandlestickConfiguration{
                candle_type: CandlestickType::TICK,
                candle_input_parameter: 1000,
            },
            true
        ).unwrap();

        let brkga = BRKGA::new(1223,36, config, FitnessExecutor::new(backtest_engine));

        assert_eq!(brkga.chromosome_size, 36);
        assert_eq!(brkga.fraction_top, 0.1);
        assert_eq!(brkga.fraction_bottom, 0.2);
        assert_eq!(brkga.population_size, 15000);
        assert_eq!(brkga.max_iterations, 1000);
        assert_eq!(brkga.elitism_rate, 0.6);
    }

    #[test]
    fn test_generate_random_individual() {
        let config: BrkgaConfig = (0.1, 0.2, 15000, 1000, 0.6);
        let backtest_engine = TradingBacktester::new(
            Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap(),
            20150102,
            20150102,
            CandlestickConfiguration{
                candle_type: CandlestickType::TICK,
                candle_input_parameter: 1000,
            },
            true
        ).unwrap();

        let mut brkga = BRKGA::new(59841, 36, config, FitnessExecutor::new(backtest_engine));
        let first_indivual = brkga.random_individual();

        assert_eq!(first_indivual.chromosome.len(), 36);

        for i in 0..first_indivual.chromosome.len() {
            assert!(first_indivual.chromosome[i] >= 0.0 && first_indivual.chromosome[i] <= 1.0);
        }

        assert_eq!(0.38724673, first_indivual.chromosome[0]);
        assert_eq!(0.65034866, first_indivual.chromosome[35]);

        let second_individual = brkga.random_individual();

        assert_ne!(first_indivual.chromosome, second_individual.chromosome);
    }
}


