#[derive(Debug)]
#[allow(unused)]
pub struct Extreme {
    pub confirmation_index: i32, // Index that this extreme point was confirmed using directional change
    pub price_index: i32,        // Index of the extreme point is inside the Vec<Trade>
    pub price: f64,
    pub height: Option<f64>, // The height of the segment between two consecutive extreme points (absolute).
    pub retrace_ratio: Option<f64>, // The ratio of the current segment's height to the previous segment's height.
    pub is_top: bool,
}

#[allow(unused)]
impl Extreme {
    // Maybe generate these values in the for inside directional change for performance?
    fn generate_retracement_ratios(extremes: &mut Vec<Extreme>) {
        // Temporários para armazenar os novos valores
        let mut heights: Vec<Option<f64>> = vec![None; extremes.len()];
        let mut retrace_ratios: Vec<Option<f64>> = vec![None; extremes.len()];

        // Primeiro passo: calcular heights e retrace_ratios
        for i in 1..extremes.len() {
            let current = &extremes[i];
            let previous = &extremes[i - 1];

            // Calcular height como a diferença absoluta nos preços
            let height = (current.price - previous.price).abs() as f64;
            heights[i] = Some(height);

            // Calcular retrace_ratio se o height anterior existir
            if let Some(prev_height) = heights[i - 1] {
                retrace_ratios[i] = Some(height / prev_height);
            }
        }

        // Segundo passo: aplicar os novos valores de volta ao vetor `extremes`
        for i in 1..extremes.len() {
            extremes[i].height = heights[i];
            extremes[i].retrace_ratio = retrace_ratios[i];
        }
    }
}
