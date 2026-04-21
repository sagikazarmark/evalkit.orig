use std::f64::consts::PI;

pub(crate) fn normalize_confidence_level(confidence_level: f64) -> f64 {
    if confidence_level.is_finite() && confidence_level > 0.0 && confidence_level < 1.0 {
        confidence_level
    } else {
        0.95
    }
}

pub(crate) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

// Student's t CDF is expressed via the regularized incomplete beta function,
// keeping confidence interval and significance calculations dependency-free.
pub(crate) fn student_t_cdf(value: f64, degrees_of_freedom: f64) -> f64 {
    if value == 0.0 {
        return 0.5;
    }

    let x = degrees_of_freedom / (degrees_of_freedom + value * value);
    let beta = regularized_incomplete_beta(x, degrees_of_freedom / 2.0, 0.5);

    if value > 0.0 {
        1.0 - 0.5 * beta
    } else {
        0.5 * beta
    }
}

pub(crate) fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }

    if x >= 1.0 {
        return 1.0;
    }

    let front =
        ((log_gamma(a + b) - log_gamma(a) - log_gamma(b)) + a * x.ln() + b * (1.0 - x).ln()).exp();

    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

pub(crate) fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    const MAX_ITERATIONS: usize = 200;
    const EPSILON: f64 = 3.0e-14;
    const FP_MIN: f64 = 1.0e-300;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;

    if d.abs() < FP_MIN {
        d = FP_MIN;
    }

    d = 1.0 / d;
    let mut h = d;

    for step in 1..=MAX_ITERATIONS {
        let step_f64 = step as f64;
        let even_numerator =
            step_f64 * (b - step_f64) * x / ((qam + 2.0 * step_f64) * (a + 2.0 * step_f64));

        d = 1.0 + even_numerator * d;
        if d.abs() < FP_MIN {
            d = FP_MIN;
        }

        c = 1.0 + even_numerator / c;
        if c.abs() < FP_MIN {
            c = FP_MIN;
        }

        d = 1.0 / d;
        h *= d * c;

        let odd_numerator = -(a + step_f64) * (qab + step_f64) * x
            / ((a + 2.0 * step_f64) * (qap + 2.0 * step_f64));

        d = 1.0 + odd_numerator * d;
        if d.abs() < FP_MIN {
            d = FP_MIN;
        }

        c = 1.0 + odd_numerator / c;
        if c.abs() < FP_MIN {
            c = FP_MIN;
        }

        d = 1.0 / d;
        let delta = d * c;
        h *= delta;

        if (delta - 1.0).abs() < EPSILON {
            break;
        }
    }

    h
}

pub(crate) fn log_gamma(value: f64) -> f64 {
    const COEFFICIENTS: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];

    if value < 0.5 {
        return PI.ln() - (PI * value).sin().ln() - log_gamma(1.0 - value);
    }

    let adjusted = value - 1.0;
    let mut series = 0.999_999_999_999_809_9;
    for (index, coefficient) in COEFFICIENTS.iter().enumerate() {
        series += coefficient / (adjusted + index as f64 + 1.0);
    }

    let t = adjusted + 7.5;
    0.5 * (2.0 * PI).ln() + (adjusted + 0.5) * t.ln() - t + series.ln()
}
