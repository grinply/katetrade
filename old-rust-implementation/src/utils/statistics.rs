use std::f64::consts::PI;

/// Coefficients used in the polynomial approximation of the CDF.
const POLY_COEFF_1: f64 = 1.330274429;
const POLY_COEFF_2: f64 = -1.821255978;
const POLY_COEFF_3: f64 = 1.781477937;
const POLY_COEFF_4: f64 = -0.356563782;
const POLY_COEFF_5: f64 = 0.319381530;
const INV_CDF_COEFF_1: f64 = 0.010328;
const INV_CDF_COEFF_2: f64 = 0.802853;
const INV_CDF_COEFF_3: f64 = 2.515517;
const INV_CDF_DENOM_1: f64 = 0.001308;
const INV_CDF_DENOM_2: f64 = 0.189269;
const INV_CDF_DENOM_3: f64 = 1.432788;

/// Coefficient used for scaling in the polynomial approximation of the CDF.
const SCALE_FACTOR: f64 = 0.2316419;

/// Computes the Cumulative Distribution Function (CDF) of the normal distribution.
/// Accurate to 7.5e-8.
pub fn normal_cdf(z: f64) -> f64 {
    let zz = z.abs();
    let pdf = (-0.5 * zz * zz).exp() / (2.0 * PI).sqrt();
    let t = 1.0 / (1.0 + zz * SCALE_FACTOR);
    let poly = ((((POLY_COEFF_1 * t + POLY_COEFF_2) * t + POLY_COEFF_3) * t + POLY_COEFF_4) * t
        + POLY_COEFF_5)
        * t;

    if z > 0.0 {
        1.0 - pdf * poly
    } else {
        pdf * poly
    }
}

/// Computes the inverse of the normal CDF.
/// Accurate to 4.5e-4.
#[allow(unused)]
pub fn inverse_normal_cdf(p: f64) -> f64 {
    let pp = if p <= 0.5 { p } else { 1.0 - p };
    let t = (1.0 / (pp * pp)).ln().sqrt();
    let numer = (INV_CDF_COEFF_1 * t + INV_CDF_COEFF_2) * t + INV_CDF_COEFF_3;
    let denom = ((INV_CDF_DENOM_1 * t + INV_CDF_DENOM_2) * t + INV_CDF_DENOM_3) * t + 1.0;
    let x = t - numer / denom;

    if p <= 0.5 {
        -x
    } else {
        x
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_inverse_normal_cdf() {
        assert_eq!(-1.221364155347629, inverse_normal_cdf(0.111));
    }
}