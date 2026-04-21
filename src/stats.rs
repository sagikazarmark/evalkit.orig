use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::math::{mean, normalize_confidence_level, student_t_cdf};
use crate::{RunResult, Score};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunStats {
    pub scorer_stats: HashMap<String, ScorerStats>,
    pub total_samples: usize,
    pub trials_per_sample: usize,
    pub total_trials_executed: usize,
    pub total_errors: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScorerStats {
    Numeric {
        mean: f64,
        stddev: f64,
        ci: (f64, f64),
        min: f64,
        max: f64,
    },
    Binary {
        pass_rate: f64,
        pass_at_k: f64,
        pass_all_k: f64,
        ci: (f64, f64),
    },
    Label {
        distribution: HashMap<String, usize>,
        mode: String,
    },
    Metric {
        mean: f64,
        stddev: f64,
        ci: (f64, f64),
        min: f64,
        max: f64,
    },
}

impl RunResult {
    pub fn stats(&self) -> RunStats {
        self.stats_with(0.95)
    }

    pub fn stats_with(&self, confidence_level: f64) -> RunStats {
        let confidence_level = normalize_confidence_level(confidence_level);
        let mut accumulators = HashMap::<String, ScorerAccumulator>::new();
        let mut total_errors = 0;

        for sample in &self.samples {
            for trial in &sample.trials {
                for (scorer_name, result) in &trial.scores {
                    match result {
                        Ok(score) => {
                            accumulators
                                .entry(scorer_name.clone())
                                .and_modify(|accumulator| {
                                    accumulator.add_score(&sample.sample_id, score)
                                })
                                .or_insert_with(|| {
                                    ScorerAccumulator::from_score(&sample.sample_id, score)
                                });
                        }
                        Err(_) => total_errors += 1,
                    }
                }
            }
        }

        let scorer_stats = accumulators
            .into_iter()
            .filter_map(|(name, accumulator)| {
                accumulator
                    .finish(confidence_level)
                    .map(|stats| (name, stats))
            })
            .collect();

        RunStats {
            scorer_stats,
            total_samples: self.samples.len(),
            trials_per_sample: self.metadata.trial_count,
            total_trials_executed: self.samples.len() * self.metadata.trial_count,
            total_errors,
        }
    }
}

impl RunStats {
    pub fn summary(&self) -> String {
        let scorer_count = self.scorer_stats.len();
        let scorer_label = if scorer_count == 1 {
            "scorer"
        } else {
            "scorers"
        };
        let trial_label = if self.trials_per_sample == 1 {
            "trial"
        } else {
            "trials"
        };

        let mut lines = vec![format!(
            "Run complete: {} samples, {} {}, {} {} per sample",
            self.total_samples, scorer_count, scorer_label, self.trials_per_sample, trial_label
        )];

        let mut scorer_names: Vec<_> = self.scorer_stats.keys().collect();
        scorer_names.sort();

        for scorer_name in scorer_names {
            let line = match self
                .scorer_stats
                .get(scorer_name)
                .expect("scorer should exist")
            {
                ScorerStats::Numeric {
                    mean,
                    stddev,
                    ci,
                    min,
                    max,
                }
                | ScorerStats::Metric {
                    mean,
                    stddev,
                    ci,
                    min,
                    max,
                } => format!(
                    "{scorer_name}: mean={mean:.2}, stddev={stddev:.2}, ci=({:.2}, {:.2}), min={min:.2}, max={max:.2}",
                    ci.0, ci.1
                ),
                ScorerStats::Binary {
                    pass_rate,
                    pass_at_k,
                    pass_all_k,
                    ci,
                } => format!(
                    "{scorer_name}: mean={pass_rate:.2}, pass_rate={:.1}%, pass_at_k={:.1}%, pass_all_k={:.1}%, ci=({:.2}, {:.2})",
                    pass_rate * 100.0,
                    pass_at_k * 100.0,
                    pass_all_k * 100.0,
                    ci.0,
                    ci.1
                ),
                ScorerStats::Label { distribution, mode } => {
                    let mut entries: Vec<_> = distribution.iter().collect();
                    entries
                        .sort_by(|(left_label, _), (right_label, _)| left_label.cmp(right_label));
                    let distribution = entries
                        .into_iter()
                        .map(|(label, count)| format!("{label}={count}"))
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!("{scorer_name}: mode={mode}, distribution={distribution}")
                }
            };

            lines.push(line);
        }

        lines.join("\n")
    }
}

enum ScorerAccumulator {
    Numeric(NumericAccumulator),
    Binary(BinaryAccumulator),
    Label(LabelAccumulator),
    Metric(NumericAccumulator),
    Mixed,
}

impl ScorerAccumulator {
    fn from_score(sample_id: &str, score: &Score) -> Self {
        let mut accumulator = match score {
            Score::Numeric(_) => Self::Numeric(NumericAccumulator::default()),
            Score::Binary(_) => Self::Binary(BinaryAccumulator::default()),
            Score::Label(_) => Self::Label(LabelAccumulator::default()),
            Score::Metric { .. } => Self::Metric(NumericAccumulator::default()),
        };

        accumulator.add_score(sample_id, score);
        accumulator
    }

    fn add_score(&mut self, sample_id: &str, score: &Score) {
        match (self, score) {
            (Self::Numeric(accumulator), Score::Numeric(value)) => accumulator.values.push(*value),
            (Self::Binary(accumulator), Score::Binary(value)) => {
                accumulator.record(sample_id, *value)
            }
            (Self::Label(accumulator), Score::Label(value)) => accumulator.record(value),
            (Self::Metric(accumulator), Score::Metric { value, .. }) => {
                accumulator.values.push(*value)
            }
            (state, _) => *state = Self::Mixed,
        }
    }

    fn finish(self, confidence_level: f64) -> Option<ScorerStats> {
        match self {
            Self::Numeric(accumulator) => Some(ScorerStats::Numeric {
                mean: mean(&accumulator.values),
                stddev: sample_stddev(&accumulator.values),
                ci: numeric_confidence_interval(&accumulator.values, confidence_level),
                min: min(&accumulator.values),
                max: max(&accumulator.values),
            }),
            Self::Binary(accumulator) => {
                let denominator = accumulator.sample_outcomes.len();
                if accumulator.scored_trials == 0 || denominator == 0 {
                    return None;
                }

                let pass_rate = accumulator.successes as f64 / accumulator.scored_trials as f64;
                let pass_at_k = accumulator
                    .sample_outcomes
                    .values()
                    .filter(|outcome| outcome.has_any_success())
                    .count() as f64
                    / denominator as f64;
                let pass_all_k = accumulator
                    .sample_outcomes
                    .values()
                    .filter(|outcome| outcome.all_successes())
                    .count() as f64
                    / denominator as f64;

                Some(ScorerStats::Binary {
                    pass_rate,
                    pass_at_k,
                    pass_all_k,
                    ci: wilson_confidence_interval(
                        accumulator.successes,
                        accumulator.scored_trials,
                        confidence_level,
                    ),
                })
            }
            Self::Label(accumulator) => {
                let mode = accumulator
                    .distribution
                    .iter()
                    .max_by(|(left_label, left_count), (right_label, right_count)| {
                        left_count
                            .cmp(right_count)
                            .then_with(|| right_label.cmp(left_label))
                    })
                    .map(|(label, _)| label.clone())?;

                Some(ScorerStats::Label {
                    distribution: accumulator.distribution,
                    mode,
                })
            }
            Self::Metric(accumulator) => Some(ScorerStats::Metric {
                mean: mean(&accumulator.values),
                stddev: sample_stddev(&accumulator.values),
                ci: numeric_confidence_interval(&accumulator.values, confidence_level),
                min: min(&accumulator.values),
                max: max(&accumulator.values),
            }),
            Self::Mixed => None,
        }
    }
}

#[derive(Default)]
struct NumericAccumulator {
    values: Vec<f64>,
}

#[derive(Default)]
struct BinaryAccumulator {
    successes: usize,
    scored_trials: usize,
    sample_outcomes: HashMap<String, BinarySampleOutcome>,
}

impl BinaryAccumulator {
    fn record(&mut self, sample_id: &str, passed: bool) {
        self.scored_trials += 1;
        if passed {
            self.successes += 1;
        }

        self.sample_outcomes
            .entry(sample_id.to_owned())
            .or_default()
            .record(passed);
    }
}

#[derive(Default)]
struct BinarySampleOutcome {
    successes: usize,
    failures: usize,
}

impl BinarySampleOutcome {
    fn record(&mut self, passed: bool) {
        if passed {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
    }

    fn has_any_success(&self) -> bool {
        self.successes > 0
    }

    fn all_successes(&self) -> bool {
        self.successes > 0 && self.failures == 0
    }
}

#[derive(Default)]
struct LabelAccumulator {
    distribution: HashMap<String, usize>,
}

impl LabelAccumulator {
    fn record(&mut self, label: &str) {
        *self.distribution.entry(label.to_owned()).or_insert(0) += 1;
    }
}

fn sample_stddev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mean = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64;

    variance.sqrt()
}

fn min(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

fn numeric_confidence_interval(values: &[f64], confidence_level: f64) -> (f64, f64) {
    let mean = mean(values);
    if values.len() < 2 {
        return (mean, mean);
    }

    let stddev = sample_stddev(values);
    let critical_value = inverse_student_t(0.5 + confidence_level / 2.0, values.len() - 1);
    let margin = critical_value * stddev / (values.len() as f64).sqrt();

    (mean - margin, mean + margin)
}

fn wilson_confidence_interval(
    successes: usize,
    trials: usize,
    confidence_level: f64,
) -> (f64, f64) {
    let proportion = successes as f64 / trials as f64;
    let z = inverse_standard_normal(0.5 + confidence_level / 2.0);
    let n = trials as f64;
    let denominator = 1.0 + (z * z / n);
    let center = (proportion + z * z / (2.0 * n)) / denominator;
    let margin = (z / denominator)
        * ((proportion * (1.0 - proportion) / n) + (z * z / (4.0 * n * n))).sqrt();

    (center - margin, center + margin)
}

fn inverse_student_t(probability: f64, degrees_of_freedom: usize) -> f64 {
    if probability == 0.5 {
        return 0.0;
    }

    if probability < 0.5 {
        return -inverse_student_t(1.0 - probability, degrees_of_freedom);
    }

    let degrees_of_freedom = degrees_of_freedom as f64;
    let mut low = 0.0;
    let mut high = inverse_standard_normal(probability).abs().max(1.0);

    while student_t_cdf(high, degrees_of_freedom) < probability {
        high *= 2.0;
    }

    for _ in 0..100 {
        let midpoint = (low + high) / 2.0;
        if student_t_cdf(midpoint, degrees_of_freedom) < probability {
            low = midpoint;
        } else {
            high = midpoint;
        }
    }

    (low + high) / 2.0
}

fn inverse_standard_normal(probability: f64) -> f64 {
    const A: [f64; 6] = [
        -39.696_830_286_653_76,
        220.946_098_424_520_5,
        -275.928_510_446_968_7,
        138.357_751_867_269,
        -30.664_798_066_147_16,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -54.476_098_798_224_06,
        161.585_836_858_040_9,
        -155.698_979_859_886_6,
        66.801_311_887_719_72,
        -13.280_681_552_885_72,
    ];
    const C: [f64; 6] = [
        -0.007_784_894_002_430_293,
        -0.322_396_458_041_136_5,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        0.007_784_695_709_041_462,
        0.322_467_129_070_039_8,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const LOW: f64 = 0.024_25;
    const HIGH: f64 = 1.0 - LOW;

    if probability <= 0.0 {
        return f64::NEG_INFINITY;
    }

    if probability >= 1.0 {
        return f64::INFINITY;
    }

    if probability < LOW {
        let q = (-2.0 * probability.ln()).sqrt();
        return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }

    if probability > HIGH {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }

    let q = probability - 0.5;
    let r = q * q;

    (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
        / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
}
