use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::{Direction, RunResult, Score};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompareConfig {
    pub confidence_level: f64,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            confidence_level: 0.95,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub baseline_id: String,
    pub candidate_id: String,
    pub shared_scorers: HashMap<String, ScorerComparison>,
    pub only_in_baseline: Vec<String>,
    pub only_in_candidate: Vec<String>,
    pub confidence_level: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScorerComparison {
    pub sample_comparisons: HashMap<String, SampleComparison>,
    pub aggregate_delta: f64,
    pub p_value: Option<f64>,
    pub significant: Option<bool>,
    pub test_used: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleComparison {
    pub sample_id: String,
    pub delta: f64,
    pub direction: Change,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    Improved,
    Regressed,
    Unchanged,
    Insignificant,
    Incomparable,
}

pub fn compare(baseline: &RunResult, candidate: &RunResult, config: CompareConfig) -> Comparison {
    let confidence_level = normalize_confidence_level(config.confidence_level);
    let baseline_definitions = score_directions(baseline);
    let candidate_definitions = score_directions(candidate);
    let baseline_scorers = scorer_names(baseline);
    let candidate_scorers = scorer_names(candidate);

    let shared_names = shared_names(&baseline_scorers, &candidate_scorers);
    let only_in_baseline = sorted_difference(&baseline_scorers, &candidate_scorers);
    let only_in_candidate = sorted_difference(&candidate_scorers, &baseline_scorers);

    let shared_scorers = shared_names
        .into_iter()
        .map(|scorer_name| {
            let baseline_scores = collect_scores(baseline, &scorer_name);
            let candidate_scores = collect_scores(candidate, &scorer_name);
            let p_value =
                significance_test(&baseline_scores.aggregate, &candidate_scores.aggregate);
            let significant = p_value.map(|value| value <= 1.0 - confidence_level);

            let sample_comparisons =
                shared_sample_ids(&baseline_scores.samples, &candidate_scores.samples)
                    .into_iter()
                    .map(|sample_id| {
                        let sample_comparison = compare_sample(
                            &sample_id,
                            baseline_scores
                                .samples
                                .get(&sample_id)
                                .expect("baseline sample exists"),
                            candidate_scores
                                .samples
                                .get(&sample_id)
                                .expect("candidate sample exists"),
                            baseline_definitions.get(&scorer_name).copied().flatten(),
                            candidate_definitions.get(&scorer_name).copied().flatten(),
                            significant,
                        );

                        (sample_id, sample_comparison)
                    })
                    .collect();

            let test_used = test_name(&baseline_scores.aggregate, &candidate_scores.aggregate);

            (
                scorer_name,
                ScorerComparison {
                    sample_comparisons,
                    aggregate_delta: aggregate_delta(
                        &baseline_scores.aggregate,
                        &candidate_scores.aggregate,
                    ),
                    p_value,
                    significant,
                    test_used,
                },
            )
        })
        .collect();

    Comparison {
        baseline_id: baseline.metadata.run_id.clone(),
        candidate_id: candidate.metadata.run_id.clone(),
        shared_scorers,
        only_in_baseline,
        only_in_candidate,
        confidence_level,
    }
}

#[derive(Default)]
struct CollectedScores {
    aggregate: ScoreBucket,
    samples: HashMap<String, ScoreBucket>,
}

#[derive(Clone, Debug, Default)]
enum ScoreBucket {
    #[default]
    Empty,
    Numeric(Vec<f64>),
    Binary(Vec<bool>),
    Label(Vec<String>),
    Metric(Vec<f64>),
    Mixed,
}

impl ScoreBucket {
    fn add_score(&mut self, score: &Score) {
        match self {
            Self::Empty => {
                *self = match score {
                    Score::Numeric(value) => Self::Numeric(vec![*value]),
                    Score::Binary(value) => Self::Binary(vec![*value]),
                    Score::Label(value) => Self::Label(vec![value.clone()]),
                    Score::Metric { value, .. } => Self::Metric(vec![*value]),
                };
            }
            Self::Numeric(values) => {
                if let Score::Numeric(value) = score {
                    values.push(*value);
                } else {
                    *self = Self::Mixed;
                }
            }
            Self::Binary(values) => {
                if let Score::Binary(value) = score {
                    values.push(*value);
                } else {
                    *self = Self::Mixed;
                }
            }
            Self::Label(values) => {
                if let Score::Label(value) = score {
                    values.push(value.clone());
                } else {
                    *self = Self::Mixed;
                }
            }
            Self::Metric(values) => {
                if let Score::Metric { value, .. } = score {
                    values.push(*value);
                } else {
                    *self = Self::Mixed;
                }
            }
            Self::Mixed => {}
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

fn scorer_names(run: &RunResult) -> HashSet<String> {
    let mut names = HashSet::new();

    for definition in &run.metadata.score_definitions {
        names.insert(definition.name.clone());
    }

    for sample in &run.samples {
        for trial in &sample.trials {
            for name in trial.scores.keys() {
                names.insert(name.clone());
            }
        }
    }

    names
}

fn score_directions(run: &RunResult) -> HashMap<String, Option<Direction>> {
    run.metadata
        .score_definitions
        .iter()
        .map(|definition| (definition.name.clone(), definition.direction))
        .collect()
}

fn shared_names(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    let mut names: Vec<_> = left.intersection(right).cloned().collect();
    names.sort();
    names
}

fn sorted_difference(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    let mut names: Vec<_> = left.difference(right).cloned().collect();
    names.sort();
    names
}

fn collect_scores(run: &RunResult, scorer_name: &str) -> CollectedScores {
    let mut collected = CollectedScores::default();

    for sample in &run.samples {
        let mut sample_bucket = ScoreBucket::default();

        for trial in &sample.trials {
            if let Some(Ok(score)) = trial.scores.get(scorer_name) {
                collected.aggregate.add_score(score);
                sample_bucket.add_score(score);
            }
        }

        if !sample_bucket.is_empty() {
            collected
                .samples
                .insert(sample.sample_id.clone(), sample_bucket);
        }
    }

    collected
}

fn shared_sample_ids(
    left: &HashMap<String, ScoreBucket>,
    right: &HashMap<String, ScoreBucket>,
) -> Vec<String> {
    let mut sample_ids: Vec<_> = left
        .keys()
        .filter(|sample_id| right.contains_key(*sample_id))
        .cloned()
        .collect();
    sample_ids.sort();
    sample_ids
}

fn compare_sample(
    sample_id: &str,
    baseline: &ScoreBucket,
    candidate: &ScoreBucket,
    baseline_direction: Option<Direction>,
    candidate_direction: Option<Direction>,
    _significant: Option<bool>,
) -> SampleComparison {
    let direction_mismatch = baseline_direction != candidate_direction;
    let (delta, direction) = match (baseline, candidate) {
        (ScoreBucket::Numeric(left), ScoreBucket::Numeric(right))
        | (ScoreBucket::Metric(left), ScoreBucket::Metric(right)) => {
            compare_ordered_series(left, right, baseline_direction, direction_mismatch)
        }
        (ScoreBucket::Binary(left), ScoreBucket::Binary(right)) => {
            compare_binary_series(left, right, baseline_direction, direction_mismatch)
        }
        (ScoreBucket::Label(left), ScoreBucket::Label(right)) => {
            compare_label_series(left, right, direction_mismatch)
        }
        _ => (0.0, Change::Incomparable),
    };

    SampleComparison {
        sample_id: sample_id.to_owned(),
        delta,
        direction,
    }
}

fn compare_ordered_series(
    baseline: &[f64],
    candidate: &[f64],
    direction: Option<Direction>,
    direction_mismatch: bool,
) -> (f64, Change) {
    if baseline.is_empty() || candidate.is_empty() || direction_mismatch {
        return (0.0, Change::Incomparable);
    }

    let delta = mean(candidate) - mean(baseline);
    (delta, direction_from_delta(delta, direction))
}

fn compare_binary_series(
    baseline: &[bool],
    candidate: &[bool],
    direction: Option<Direction>,
    direction_mismatch: bool,
) -> (f64, Change) {
    if baseline.is_empty() || candidate.is_empty() || direction_mismatch {
        return (0.0, Change::Incomparable);
    }

    let baseline_rate = binary_pass_rate(baseline);
    let candidate_rate = binary_pass_rate(candidate);
    let delta = candidate_rate - baseline_rate;

    (delta, direction_from_delta(delta, direction))
}

fn compare_label_series(
    baseline: &[String],
    candidate: &[String],
    direction_mismatch: bool,
) -> (f64, Change) {
    if baseline.is_empty() || candidate.is_empty() || direction_mismatch {
        return (0.0, Change::Incomparable);
    }

    let baseline_mode = mode(baseline);
    let candidate_mode = mode(candidate);

    if baseline_mode.is_none() || candidate_mode.is_none() {
        return (0.0, Change::Incomparable);
    }

    if baseline_mode == candidate_mode {
        (0.0, Change::Unchanged)
    } else {
        (0.0, Change::Incomparable)
    }
}

fn aggregate_delta(baseline: &ScoreBucket, candidate: &ScoreBucket) -> f64 {
    match (baseline, candidate) {
        (ScoreBucket::Numeric(left), ScoreBucket::Numeric(right))
        | (ScoreBucket::Metric(left), ScoreBucket::Metric(right))
            if !left.is_empty() && !right.is_empty() =>
        {
            mean(right) - mean(left)
        }
        (ScoreBucket::Binary(left), ScoreBucket::Binary(right))
            if !left.is_empty() && !right.is_empty() =>
        {
            binary_pass_rate(right) - binary_pass_rate(left)
        }
        (ScoreBucket::Label(_), ScoreBucket::Label(_)) => 0.0,
        _ => 0.0,
    }
}

fn significance_test(baseline: &ScoreBucket, candidate: &ScoreBucket) -> Option<f64> {
    match (baseline, candidate) {
        (ScoreBucket::Numeric(left), ScoreBucket::Numeric(right))
        | (ScoreBucket::Metric(left), ScoreBucket::Metric(right)) => {
            welch_t_test_p_value(left, right)
        }
        (ScoreBucket::Binary(left), ScoreBucket::Binary(right)) => {
            fisher_exact_test_p_value(left, right)
        }
        _ => None,
    }
}

fn test_name(baseline: &ScoreBucket, candidate: &ScoreBucket) -> Option<String> {
    match (baseline, candidate) {
        (ScoreBucket::Numeric(left), ScoreBucket::Numeric(right))
        | (ScoreBucket::Metric(left), ScoreBucket::Metric(right))
            if left.len() >= 2 && right.len() >= 2 =>
        {
            Some("welch_t_test".to_owned())
        }
        (ScoreBucket::Binary(left), ScoreBucket::Binary(right))
            if !left.is_empty() && !right.is_empty() =>
        {
            Some("fisher_exact_test".to_owned())
        }
        _ => None,
    }
}

fn direction_from_delta(delta: f64, direction: Option<Direction>) -> Change {
    if delta.abs() <= f64::EPSILON {
        return Change::Unchanged;
    }

    match direction.unwrap_or(Direction::Maximize) {
        Direction::Maximize => {
            if delta > 0.0 {
                Change::Improved
            } else {
                Change::Regressed
            }
        }
        Direction::Minimize => {
            if delta < 0.0 {
                Change::Improved
            } else {
                Change::Regressed
            }
        }
    }
}

fn binary_pass_rate(values: &[bool]) -> f64 {
    values.iter().filter(|value| **value).count() as f64 / values.len() as f64
}

fn mode(values: &[String]) -> Option<String> {
    let mut distribution = HashMap::<&str, usize>::new();
    for value in values {
        *distribution.entry(value.as_str()).or_insert(0) += 1;
    }

    distribution
        .into_iter()
        .max_by(|(left_label, left_count), (right_label, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_label.cmp(left_label))
        })
        .map(|(label, _)| label.to_owned())
}

fn normalize_confidence_level(confidence_level: f64) -> f64 {
    if confidence_level.is_finite() && confidence_level > 0.0 && confidence_level < 1.0 {
        confidence_level
    } else {
        0.95
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mean = mean(values);
    values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn welch_t_test_p_value(baseline: &[f64], candidate: &[f64]) -> Option<f64> {
    if baseline.len() < 2 || candidate.len() < 2 {
        return None;
    }

    let baseline_variance = sample_variance(baseline);
    let candidate_variance = sample_variance(candidate);
    let baseline_size = baseline.len() as f64;
    let candidate_size = candidate.len() as f64;
    let standard_error =
        (baseline_variance / baseline_size + candidate_variance / candidate_size).sqrt();
    let mean_delta = mean(candidate) - mean(baseline);

    if standard_error == 0.0 {
        return Some(if mean_delta.abs() <= f64::EPSILON {
            1.0
        } else {
            0.0
        });
    }

    let t_statistic = mean_delta.abs() / standard_error;
    let numerator =
        (baseline_variance / baseline_size + candidate_variance / candidate_size).powi(2);
    let denominator = if baseline.len() > 1 {
        (baseline_variance / baseline_size).powi(2) / (baseline_size - 1.0)
    } else {
        0.0
    } + if candidate.len() > 1 {
        (candidate_variance / candidate_size).powi(2) / (candidate_size - 1.0)
    } else {
        0.0
    };

    if denominator == 0.0 {
        return None;
    }

    let degrees_of_freedom = numerator / denominator;
    Some((2.0 * (1.0 - student_t_cdf(t_statistic, degrees_of_freedom))).clamp(0.0, 1.0))
}

fn fisher_exact_test_p_value(baseline: &[bool], candidate: &[bool]) -> Option<f64> {
    if baseline.is_empty() || candidate.is_empty() {
        return None;
    }

    let baseline_successes = baseline.iter().filter(|value| **value).count();
    let baseline_failures = baseline.len() - baseline_successes;
    let candidate_successes = candidate.iter().filter(|value| **value).count();
    let candidate_failures = candidate.len() - candidate_successes;

    Some(fisher_exact_p_value(
        baseline_successes,
        baseline_failures,
        candidate_successes,
        candidate_failures,
    ))
}

fn fisher_exact_p_value(a: usize, b: usize, c: usize, d: usize) -> f64 {
    let row_one = a + b;
    let row_two = c + d;
    let column_one = a + c;
    let column_two = b + d;
    let min_a = row_one.saturating_sub(column_two);
    let max_a = row_one.min(column_one);
    let observed = hypergeometric_probability(a, row_one, row_two, column_one);
    let mut p_value = 0.0;

    for possible_a in min_a..=max_a {
        let probability = hypergeometric_probability(possible_a, row_one, row_two, column_one);
        if probability <= observed + 1e-12 {
            p_value += probability;
        }
    }

    p_value.clamp(0.0, 1.0)
}

fn hypergeometric_probability(a: usize, row_one: usize, row_two: usize, column_one: usize) -> f64 {
    let total = row_one + row_two;
    let log_probability = log_combination(row_one, a) + log_combination(row_two, column_one - a)
        - log_combination(total, column_one);

    log_probability.exp()
}

fn log_combination(n: usize, k: usize) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }

    log_gamma((n + 1) as f64) - log_gamma((k + 1) as f64) - log_gamma((n - k + 1) as f64)
}

fn student_t_cdf(value: f64, degrees_of_freedom: f64) -> f64 {
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

fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
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

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
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

fn log_gamma(value: f64) -> f64 {
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
