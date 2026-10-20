//! Gaussian kernel density estimation with Silverman's rule-of-thumb
//! bandwidth, used by `plot::violin` to turn a group's raw values into a
//! smooth density curve for its violin shape.

use std::f64::consts::PI;

/// Silverman's rule-of-thumb bandwidth for `samples`:
/// `0.9 * min(std_dev, iqr / 1.34) * n.powf(-0.2)`, matching SciPy's/
/// `matplotlib`'s (via `seaborn`) default closely enough to produce a
/// recognizable violin shape.
///
/// `std_dev` is the sample standard deviation with Bessel's correction
/// (divided by `n - 1`). `iqr` uses linear interpolation between order
/// statistics, matching NumPy's default percentile interpolation.
/// Returns `0.0` when `samples` has fewer than two elements, or when
/// every sample is identical (no spread to estimate a bandwidth from).
fn silverman_bandwidth(samples: &[f64]) -> f64 {
    let n = samples.len();
    if n < 2 {
        return 0.0;
    }
    let n_f = n as f64;
    let mean = samples.iter().sum::<f64>() / n_f;
    let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n_f - 1.0);
    let std_dev = variance.sqrt();

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("samples must not be NaN"));
    let iqr = percentile(&sorted, 0.75) - percentile(&sorted, 0.25);

    let spread = if iqr > 0.0 {
        std_dev.min(iqr / 1.34)
    } else {
        std_dev
    };
    0.9 * spread * n_f.powf(-0.2)
}

/// Linear-interpolated percentile of a pre-sorted slice (`q` in `[0,
/// 1]`), matching NumPy's default (`linear`) interpolation method.
fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = q * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = idx - lo as f64;
        sorted[lo] + (sorted[hi] - sorted[lo]) * frac
    }
}

/// Standard normal probability density function.
fn normal_pdf(u: f64) -> f64 {
    (-0.5 * u * u).exp() / (2.0 * PI).sqrt()
}

/// Gaussian kernel density estimate of `samples`, evaluated at each of
/// `eval_points`, using Silverman's rule-of-thumb bandwidth.
///
/// Returns one density value per `eval_points` entry, in the same
/// order. Returns all zeros when `samples` is empty or the computed
/// bandwidth is zero (fewer than two samples, or every sample
/// identical) since no density curve can be estimated in that case.
pub fn gaussian_kde(samples: &[f64], eval_points: &[f64]) -> Vec<f64> {
    let n = samples.len();
    if n == 0 {
        return vec![0.0; eval_points.len()];
    }
    let h = silverman_bandwidth(samples);
    if h <= 0.0 {
        return vec![0.0; eval_points.len()];
    }
    let n_f = n as f64;
    eval_points
        .iter()
        .map(|&x| {
            samples
                .iter()
                .map(|&xi| normal_pdf((x - xi) / h))
                .sum::<f64>()
                / (n_f * h)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cross-check for the `samples = [1.0, 2.0, 3.0]` case below: an
    // independent Node.js script (not calling into this module)
    // implementing the same published formulas — Silverman's rule of
    // thumb (`0.9 * min(std_dev, iqr / 1.34) * n^-0.2`, std_dev with
    // Bessel's correction, iqr via linear-interpolated percentiles) and
    // the standard Gaussian kernel `exp(-u^2/2) / sqrt(2*pi)` — gives:
    //   mean=2, variance=1, std_dev=1, q1=1.5, q3=2.5, iqr=1
    //   bandwidth h=0.539154780286722
    //   kde(1.0) = 0.29106377172773107
    //   kde(2.0) = 0.33497376061036666
    //   kde(3.0) = 0.29106377172773107
    //   kde(0.0) = 0.044417094673962605
    //   kde(4.0) = 0.04441709467396261
    #[test]
    fn gaussian_kde_matches_independent_cross_check() {
        let samples = [1.0, 2.0, 3.0];
        let eval_points = [1.0, 2.0, 3.0, 0.0, 4.0];
        let expected = [
            0.29106377172773107,
            0.33497376061036666,
            0.29106377172773107,
            0.044417094673962605,
            0.04441709467396261,
        ];
        let densities = gaussian_kde(&samples, &eval_points);
        assert_eq!(densities.len(), expected.len());
        for (got, want) in densities.iter().zip(expected.iter()) {
            assert!(
                (got - want).abs() < 1e-9,
                "got {got}, want {want} (within 1e-9)"
            );
        }
    }

    #[test]
    fn gaussian_kde_is_symmetric_around_the_mean_for_symmetric_samples() {
        let samples = [1.0, 2.0, 3.0];
        let densities = gaussian_kde(&samples, &[1.0, 3.0]);
        assert!((densities[0] - densities[1]).abs() < 1e-12);
    }

    #[test]
    fn gaussian_kde_returns_zeros_for_empty_samples() {
        let densities = gaussian_kde(&[], &[0.0, 1.0]);
        assert_eq!(densities, vec![0.0, 0.0]);
    }

    #[test]
    fn gaussian_kde_returns_zeros_when_all_samples_are_identical() {
        let densities = gaussian_kde(&[5.0, 5.0, 5.0], &[5.0, 6.0]);
        assert_eq!(densities, vec![0.0, 0.0]);
    }
}
