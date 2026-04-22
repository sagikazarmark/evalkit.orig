use std::collections::HashMap;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use evalkit::{
    RunMetadata, RunResult, SampleResult, Score, ScoreDefinition, ScorerError, ScorerStats,
    TrialResult,
};

fn metadata(score_definitions: Vec<ScoreDefinition>, trial_count: usize) -> RunMetadata {
    RunMetadata {
        run_id: "run-123".to_owned(),
        seed: None,
        dataset_fingerprint: "dataset-stats".to_owned(),
        scorer_fingerprint: "scorers-stats".to_owned(),
        started_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
        completed_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 5).unwrap(),
        duration: Duration::from_secs(5),
        trial_count,
        score_definitions,
        acquisition_mode: "inline".to_owned(),
    }
}

fn sample(sample_id: &str, trials: Vec<TrialResult>) -> SampleResult {
    let scored_count = trials
        .iter()
        .filter(|trial| trial.scores.values().any(Result::is_ok))
        .count();

    SampleResult {
        sample_id: sample_id.to_owned(),
        trial_count: trials.len(),
        scored_count,
        error_count: trials.len() - scored_count,
        token_usage: Default::default(),
        cost_usd: None,
        trials,
    }
}

fn trial(score_name: &str, result: Result<Score, ScorerError>, trial_index: usize) -> TrialResult {
    TrialResult {
        scores: HashMap::from([(score_name.to_owned(), result)]),
        duration: Duration::from_millis(10 + trial_index as u64),
        trial_index,
    }
}

fn assert_close(left: f64, right: f64) {
    let delta = (left - right).abs();
    assert!(
        delta <= 1e-9,
        "expected {left} to be within 1e-9 of {right}, delta was {delta}"
    );
}

#[test]
fn run_result_stats_compute_numeric_and_metric_aggregates_with_bootstrap_intervals() {
    let run = RunResult {
        metadata: metadata(
            vec![
                ScoreDefinition::maximize("accuracy"),
                ScoreDefinition::minimize("latency"),
            ],
            2,
        ),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    TrialResult {
                        scores: HashMap::from([
                            ("accuracy".to_owned(), Ok(Score::Numeric(1.0))),
                            (
                                "latency".to_owned(),
                                Ok(Score::Metric {
                                    name: "latency".to_owned(),
                                    value: 100.0,
                                    unit: Some("ms".to_owned()),
                                }),
                            ),
                        ]),
                        duration: Duration::from_millis(10),
                        trial_index: 0,
                    },
                    TrialResult {
                        scores: HashMap::from([
                            ("accuracy".to_owned(), Ok(Score::Numeric(2.0))),
                            (
                                "latency".to_owned(),
                                Ok(Score::Metric {
                                    name: "latency".to_owned(),
                                    value: 110.0,
                                    unit: Some("ms".to_owned()),
                                }),
                            ),
                        ]),
                        duration: Duration::from_millis(11),
                        trial_index: 1,
                    },
                ],
            ),
            sample(
                "sample-b",
                vec![
                    TrialResult {
                        scores: HashMap::from([
                            ("accuracy".to_owned(), Ok(Score::Numeric(3.0))),
                            (
                                "latency".to_owned(),
                                Ok(Score::Metric {
                                    name: "latency".to_owned(),
                                    value: 90.0,
                                    unit: Some("ms".to_owned()),
                                }),
                            ),
                        ]),
                        duration: Duration::from_millis(12),
                        trial_index: 0,
                    },
                    TrialResult {
                        scores: HashMap::from([
                            ("accuracy".to_owned(), Ok(Score::Numeric(4.0))),
                            (
                                "latency".to_owned(),
                                Ok(Score::Metric {
                                    name: "latency".to_owned(),
                                    value: 95.0,
                                    unit: Some("ms".to_owned()),
                                }),
                            ),
                        ]),
                        duration: Duration::from_millis(13),
                        trial_index: 1,
                    },
                ],
            ),
        ],
    };

    let stats = run.stats();

    assert_eq!(stats.total_samples, 2);
    assert_eq!(stats.trials_per_sample, 2);
    assert_eq!(stats.total_errors, 0);

    match stats.scorer_stats.get("accuracy").expect("accuracy stats") {
        ScorerStats::Numeric {
            mean,
            stddev,
            ci,
            min,
            max,
        } => {
            assert_close(*mean, 2.5);
            assert_close(*stddev, 1.290_994_448_735_805_6);
            assert!(ci.0 <= *mean && *mean <= ci.1);
            assert!((1.0..=4.0).contains(&ci.0));
            assert!((1.0..=4.0).contains(&ci.1));
            assert_close(*min, 1.0);
            assert_close(*max, 4.0);
        }
        other => panic!("unexpected accuracy stats: {other:?}"),
    }

    match stats.scorer_stats.get("latency").expect("latency stats") {
        ScorerStats::Metric {
            mean,
            stddev,
            ci,
            min,
            max,
        } => {
            assert_close(*mean, 98.75);
            assert_close(*stddev, 8.539_125_638_299_666);
            assert!(ci.0 <= *mean && *mean <= ci.1);
            assert!((90.0..=110.0).contains(&ci.0));
            assert!((90.0..=110.0).contains(&ci.1));
            assert_close(*min, 90.0);
            assert_close(*max, 110.0);
        }
        other => panic!("unexpected latency stats: {other:?}"),
    }
}

#[test]
fn run_result_stats_compute_binary_pass_metrics_and_wilson_interval() {
    let run = RunResult {
        metadata: metadata(vec![ScoreDefinition::maximize("exact_match")], 2),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial("exact_match", Ok(Score::Binary(true)), 0),
                    trial("exact_match", Ok(Score::Binary(false)), 1),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial("exact_match", Ok(Score::Binary(true)), 0),
                    trial("exact_match", Ok(Score::Binary(true)), 1),
                ],
            ),
        ],
    };

    let stats = run.stats();

    match stats
        .scorer_stats
        .get("exact_match")
        .expect("binary stats should exist")
    {
        ScorerStats::Binary {
            pass_rate,
            pass_at_k,
            pass_all_k,
            ci,
        } => {
            assert_close(*pass_rate, 0.75);
            assert_close(*pass_at_k, 1.0);
            assert_close(*pass_all_k, 0.5);
            assert_close(ci.0, 0.300_641_842_328_362_9);
            assert_close(ci.1, 0.954_412_739_242_868_6);
        }
        other => panic!("unexpected exact_match stats: {other:?}"),
    }

    let summary = stats.summary();
    assert!(summary.contains("Run complete: 2 samples, 1 scorer, 2 trials"));
    assert!(summary.contains("exact_match: mean=0.75, pass_rate=75.0%"));
}

#[test]
fn run_result_stats_compute_label_distribution_and_mode() {
    let run = RunResult {
        metadata: metadata(vec![ScoreDefinition::new("topic")], 2),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial("topic", Ok(Score::Label("math".to_owned())), 0),
                    trial("topic", Ok(Score::Label("science".to_owned())), 1),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial("topic", Ok(Score::Label("math".to_owned())), 0),
                    trial("topic", Ok(Score::Label("math".to_owned())), 1),
                ],
            ),
        ],
    };

    let stats = run.stats();

    match stats.scorer_stats.get("topic").expect("label stats") {
        ScorerStats::Label { distribution, mode } => {
            assert_eq!(distribution.get("math"), Some(&3));
            assert_eq!(distribution.get("science"), Some(&1));
            assert_eq!(mode, "math");
        }
        other => panic!("unexpected label stats: {other:?}"),
    }
}

#[test]
fn run_result_stats_exclude_errors_from_denominator_and_report_total_errors() {
    let run = RunResult {
        metadata: metadata(vec![ScoreDefinition::maximize("exact_match")], 2),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial("exact_match", Ok(Score::Binary(true)), 0),
                    trial(
                        "exact_match",
                        Err(ScorerError::internal(std::io::Error::other("timeout"))),
                        1,
                    ),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial("exact_match", Ok(Score::Binary(false)), 0),
                    trial(
                        "exact_match",
                        Err(ScorerError::internal(std::io::Error::other("backend"))),
                        1,
                    ),
                ],
            ),
        ],
    };

    let stats = run.stats();

    assert_eq!(stats.total_errors, 2);

    match stats
        .scorer_stats
        .get("exact_match")
        .expect("binary stats should exist")
    {
        ScorerStats::Binary {
            pass_rate,
            pass_at_k,
            pass_all_k,
            ..
        } => {
            assert_close(*pass_rate, 0.5);
            assert_close(*pass_at_k, 0.5);
            assert_close(*pass_all_k, 0.5);
        }
        other => panic!("unexpected exact_match stats: {other:?}"),
    }
}

#[test]
fn run_result_stats_handle_single_trial_without_special_casing() {
    let run = RunResult {
        metadata: metadata(vec![ScoreDefinition::maximize("exact_match")], 1),
        samples: vec![sample(
            "sample-a",
            vec![trial("exact_match", Ok(Score::Binary(true)), 0)],
        )],
    };

    let stats = run.stats_with(0.9);

    assert_eq!(stats.total_samples, 1);
    assert_eq!(stats.trials_per_sample, 1);
    assert_eq!(stats.total_errors, 0);

    match stats
        .scorer_stats
        .get("exact_match")
        .expect("binary stats should exist")
    {
        ScorerStats::Binary {
            pass_rate,
            pass_at_k,
            pass_all_k,
            ci,
        } => {
            assert_close(*pass_rate, 1.0);
            assert_close(*pass_at_k, 1.0);
            assert_close(*pass_all_k, 1.0);
            assert!(ci.0 < 1.0);
            assert_close(ci.1, 1.0);
        }
        other => panic!("unexpected exact_match stats: {other:?}"),
    }
}
