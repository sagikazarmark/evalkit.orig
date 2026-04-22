use std::time::Duration;

use chrono::{TimeZone, Utc};
use evalkit::{
    Change, CompareConfig, RunMetadata, RunResult, SampleComparison, SampleResult, Score,
    ScoreDefinition, TrialResult, compare,
};

fn metadata(
    run_id: &str,
    score_definitions: Vec<ScoreDefinition>,
    trial_count: usize,
) -> RunMetadata {
    RunMetadata {
        run_id: run_id.to_owned(),
        seed: None,
        dataset_fingerprint: format!("dataset-{run_id}"),
        scorer_fingerprint: format!("scorers-{run_id}"),
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

fn trial(entries: Vec<(&str, Score)>, trial_index: usize) -> TrialResult {
    TrialResult {
        scores: entries
            .into_iter()
            .map(|(name, score)| (name.to_owned(), Ok(score)))
            .collect(),
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
fn compare_reports_direction_aware_numeric_and_metric_sample_deltas() {
    let baseline = RunResult {
        metadata: metadata(
            "baseline",
            vec![
                ScoreDefinition::maximize("accuracy"),
                ScoreDefinition::minimize("latency"),
            ],
            1,
        ),
        samples: vec![
            sample(
                "sample-a",
                vec![trial(
                    vec![
                        ("accuracy", Score::Numeric(0.40)),
                        (
                            "latency",
                            Score::Metric {
                                name: "latency".to_owned(),
                                value: 120.0,
                                unit: Some("ms".to_owned()),
                            },
                        ),
                    ],
                    0,
                )],
            ),
            sample(
                "sample-b",
                vec![trial(
                    vec![
                        ("accuracy", Score::Numeric(0.90)),
                        (
                            "latency",
                            Score::Metric {
                                name: "latency".to_owned(),
                                value: 80.0,
                                unit: Some("ms".to_owned()),
                            },
                        ),
                    ],
                    0,
                )],
            ),
        ],
    };

    let candidate = RunResult {
        metadata: metadata(
            "candidate",
            vec![
                ScoreDefinition::maximize("accuracy"),
                ScoreDefinition::minimize("latency"),
            ],
            1,
        ),
        samples: vec![
            sample(
                "sample-a",
                vec![trial(
                    vec![
                        ("accuracy", Score::Numeric(0.60)),
                        (
                            "latency",
                            Score::Metric {
                                name: "latency".to_owned(),
                                value: 100.0,
                                unit: Some("ms".to_owned()),
                            },
                        ),
                    ],
                    0,
                )],
            ),
            sample(
                "sample-b",
                vec![trial(
                    vec![
                        ("accuracy", Score::Numeric(0.80)),
                        (
                            "latency",
                            Score::Metric {
                                name: "latency".to_owned(),
                                value: 90.0,
                                unit: Some("ms".to_owned()),
                            },
                        ),
                    ],
                    0,
                )],
            ),
        ],
    };

    let comparison = compare(&baseline, &candidate, CompareConfig::default());

    assert_eq!(comparison.baseline_id, "baseline");
    assert_eq!(comparison.candidate_id, "candidate");
    assert!(comparison.only_in_baseline.is_empty());
    assert!(comparison.only_in_candidate.is_empty());

    let accuracy = comparison
        .shared_scorers
        .get("accuracy")
        .expect("accuracy comparison");
    assert_close(accuracy.aggregate_delta, 0.05);
    assert_eq!(accuracy.test_used.as_deref(), Some("paired_t_test"));
    let accuracy_sample_a = accuracy
        .sample_comparisons
        .get("sample-a")
        .expect("sample-a comparison should exist");
    assert_eq!(accuracy_sample_a.sample_id, "sample-a");
    assert_close(accuracy_sample_a.delta, 0.20);
    // aggregate test is non-significant with only 2 samples per group
    assert_eq!(accuracy_sample_a.direction, Change::Insignificant);

    let accuracy_sample_b = accuracy
        .sample_comparisons
        .get("sample-b")
        .expect("sample-b comparison should exist");
    assert_eq!(accuracy_sample_b.sample_id, "sample-b");
    assert_close(accuracy_sample_b.delta, -0.10);
    assert_eq!(accuracy_sample_b.direction, Change::Insignificant);

    let latency = comparison
        .shared_scorers
        .get("latency")
        .expect("latency comparison");
    assert_close(latency.aggregate_delta, -5.0);
    let latency_sample_a = latency
        .sample_comparisons
        .get("sample-a")
        .expect("sample-a latency comparison should exist");
    assert_eq!(latency_sample_a.sample_id, "sample-a");
    assert_close(latency_sample_a.delta, -20.0);
    assert_eq!(latency_sample_a.direction, Change::Insignificant);

    let latency_sample_b = latency
        .sample_comparisons
        .get("sample-b")
        .expect("sample-b latency comparison should exist");
    assert_eq!(latency_sample_b.sample_id, "sample-b");
    assert_close(latency_sample_b.delta, 10.0);
    assert_eq!(latency_sample_b.direction, Change::Insignificant);
}

#[test]
fn compare_uses_paired_t_test_and_marks_non_significant_deltas() {
    let baseline = RunResult {
        metadata: metadata("baseline", vec![ScoreDefinition::maximize("accuracy")], 2),
        samples: vec![sample(
            "sample-a",
            vec![
                trial(vec![("accuracy", Score::Numeric(0.40))], 0),
                trial(vec![("accuracy", Score::Numeric(0.60))], 1),
            ],
        )],
    };

    let candidate = RunResult {
        metadata: metadata("candidate", vec![ScoreDefinition::maximize("accuracy")], 2),
        samples: vec![sample(
            "sample-a",
            vec![
                trial(vec![("accuracy", Score::Numeric(0.45))], 0),
                trial(vec![("accuracy", Score::Numeric(0.62))], 1),
            ],
        )],
    };

    let comparison = compare(&baseline, &candidate, CompareConfig::default());
    let accuracy = comparison
        .shared_scorers
        .get("accuracy")
        .expect("accuracy comparison");

    assert_eq!(accuracy.test_used.as_deref(), Some("paired_t_test"));
    assert_eq!(accuracy.significant, Some(false));
    assert!(accuracy.p_value.expect("p-value should exist") > 0.05);
    let sample_comparison = accuracy
        .sample_comparisons
        .get("sample-a")
        .expect("sample comparison should exist");
    assert_eq!(sample_comparison.sample_id, "sample-a");
    assert_close(sample_comparison.delta, 0.035);
    assert_eq!(sample_comparison.direction, Change::Insignificant);
}

#[test]
fn compare_applies_the_configured_confidence_level_to_significance() {
    let baseline = RunResult {
        metadata: metadata("baseline", vec![ScoreDefinition::maximize("accuracy")], 2),
        samples: vec![sample(
            "sample-a",
            vec![
                trial(vec![("accuracy", Score::Numeric(0.40))], 0),
                trial(vec![("accuracy", Score::Numeric(0.60))], 1),
            ],
        )],
    };

    let candidate = RunResult {
        metadata: metadata("candidate", vec![ScoreDefinition::maximize("accuracy")], 2),
        samples: vec![sample(
            "sample-a",
            vec![
                trial(vec![("accuracy", Score::Numeric(0.45))], 0),
                trial(vec![("accuracy", Score::Numeric(0.65))], 1),
            ],
        )],
    };

    let strict = compare(&baseline, &candidate, CompareConfig::default());
    let relaxed = compare(
        &baseline,
        &candidate,
        CompareConfig {
            confidence_level: 0.80,
        },
    );

    let strict_accuracy = strict
        .shared_scorers
        .get("accuracy")
        .expect("strict comparison");
    let relaxed_accuracy = relaxed
        .shared_scorers
        .get("accuracy")
        .expect("relaxed comparison");
    let p_value = relaxed_accuracy.p_value.expect("p-value should exist");

    assert_close(strict.confidence_level, 0.95);
    assert_close(relaxed.confidence_level, 0.80);
    assert_eq!(strict_accuracy.significant, Some(p_value <= 0.05));
    assert_eq!(relaxed_accuracy.significant, Some(p_value <= 0.20));
}

#[test]
fn compare_reports_improved_and_regressed_when_aggregate_is_significant() {
    // Many tightly-clustered trials produce a statistically significant result,
    // which lets per-sample direction labels reach Improved/Regressed.
    let baseline = RunResult {
        metadata: metadata("baseline", vec![ScoreDefinition::maximize("accuracy")], 10),
        samples: vec![
            sample(
                "sample-a",
                (0..10)
                    .map(|i| trial(vec![("accuracy", Score::Numeric(0.40 + i as f64 * 0.001))], i))
                    .collect(),
            ),
            sample(
                "sample-b",
                (0..10)
                    .map(|i| trial(vec![("accuracy", Score::Numeric(0.42 + i as f64 * 0.001))], i))
                    .collect(),
            ),
        ],
    };

    let candidate = RunResult {
        metadata: metadata("candidate", vec![ScoreDefinition::maximize("accuracy")], 10),
        samples: vec![
            sample(
                "sample-a",
                (0..10)
                    .map(|i| trial(vec![("accuracy", Score::Numeric(0.80 + i as f64 * 0.001))], i))
                    .collect(),
            ),
            sample(
                "sample-b",
                (0..10)
                    .map(|i| trial(vec![("accuracy", Score::Numeric(0.82 + i as f64 * 0.001))], i))
                    .collect(),
            ),
        ],
    };

    let comparison = compare(&baseline, &candidate, CompareConfig::default());
    let accuracy = comparison
        .shared_scorers
        .get("accuracy")
        .expect("accuracy comparison");

    assert_eq!(accuracy.significant, Some(true));

    let sample_a = accuracy
        .sample_comparisons
        .get("sample-a")
        .expect("sample-a comparison");
    assert_eq!(sample_a.direction, Change::Improved);

    let sample_b = accuracy
        .sample_comparisons
        .get("sample-b")
        .expect("sample-b comparison");
    assert_eq!(sample_b.direction, Change::Improved);
}

#[test]
fn compare_uses_fisher_exact_for_binary_scores_with_different_trial_counts() {
    let baseline = RunResult {
        metadata: metadata(
            "baseline",
            vec![ScoreDefinition::maximize("exact_match")],
            2,
        ),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial(vec![("exact_match", Score::Binary(false))], 0),
                    trial(vec![("exact_match", Score::Binary(false))], 1),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial(vec![("exact_match", Score::Binary(false))], 0),
                    trial(vec![("exact_match", Score::Binary(false))], 1),
                ],
            ),
        ],
    };

    let candidate = RunResult {
        metadata: metadata(
            "candidate",
            vec![ScoreDefinition::maximize("exact_match")],
            3,
        ),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial(vec![("exact_match", Score::Binary(true))], 0),
                    trial(vec![("exact_match", Score::Binary(true))], 1),
                    trial(vec![("exact_match", Score::Binary(true))], 2),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial(vec![("exact_match", Score::Binary(true))], 0),
                    trial(vec![("exact_match", Score::Binary(true))], 1),
                    trial(vec![("exact_match", Score::Binary(true))], 2),
                ],
            ),
        ],
    };

    let comparison = compare(&baseline, &candidate, CompareConfig::default());
    let exact_match = comparison
        .shared_scorers
        .get("exact_match")
        .expect("exact_match comparison");

    assert_eq!(exact_match.test_used.as_deref(), Some("fisher_exact_test"));
    assert_eq!(exact_match.significant, Some(true));
    assert!(exact_match.p_value.expect("p-value should exist") < 0.05);
    assert_close(exact_match.aggregate_delta, 1.0);
    assert_eq!(
        exact_match.sample_comparisons.get("sample-a"),
        Some(&SampleComparison {
            sample_id: "sample-a".to_owned(),
            delta: 1.0,
            direction: Change::Improved,
        })
    );
}

#[test]
fn compare_marks_direction_mismatch_as_incomparable() {
    let baseline = RunResult {
        metadata: metadata("baseline", vec![ScoreDefinition::maximize("latency")], 1),
        samples: vec![sample(
            "sample-a",
            vec![trial(vec![("latency", Score::Numeric(100.0))], 0)],
        )],
    };

    let candidate = RunResult {
        metadata: metadata("candidate", vec![ScoreDefinition::minimize("latency")], 1),
        samples: vec![sample(
            "sample-a",
            vec![trial(vec![("latency", Score::Numeric(90.0))], 0)],
        )],
    };

    let comparison = compare(&baseline, &candidate, CompareConfig::default());
    let latency = comparison
        .shared_scorers
        .get("latency")
        .expect("latency comparison");

    assert_eq!(
        latency.sample_comparisons.get("sample-a"),
        Some(&SampleComparison {
            sample_id: "sample-a".to_owned(),
            delta: 0.0,
            direction: Change::Incomparable,
        })
    );
}

#[test]
fn compare_treats_label_scores_as_change_only_without_significance() {
    let baseline = RunResult {
        metadata: metadata("baseline", vec![ScoreDefinition::new("topic")], 2),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial(vec![("topic", Score::Label("math".to_owned()))], 0),
                    trial(vec![("topic", Score::Label("math".to_owned()))], 1),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial(vec![("topic", Score::Label("science".to_owned()))], 0),
                    trial(vec![("topic", Score::Label("science".to_owned()))], 1),
                ],
            ),
        ],
    };

    let candidate = RunResult {
        metadata: metadata("candidate", vec![ScoreDefinition::new("topic")], 2),
        samples: vec![
            sample(
                "sample-a",
                vec![
                    trial(vec![("topic", Score::Label("math".to_owned()))], 0),
                    trial(vec![("topic", Score::Label("math".to_owned()))], 1),
                ],
            ),
            sample(
                "sample-b",
                vec![
                    trial(vec![("topic", Score::Label("history".to_owned()))], 0),
                    trial(vec![("topic", Score::Label("history".to_owned()))], 1),
                ],
            ),
        ],
    };

    let comparison = compare(&baseline, &candidate, CompareConfig::default());
    let topic = comparison
        .shared_scorers
        .get("topic")
        .expect("topic comparison");

    assert_eq!(topic.aggregate_delta, 0.0);
    assert_eq!(topic.p_value, None);
    assert_eq!(topic.significant, None);
    assert_eq!(topic.test_used, None);
    assert_eq!(
        topic.sample_comparisons.get("sample-a"),
        Some(&SampleComparison {
            sample_id: "sample-a".to_owned(),
            delta: 0.0,
            direction: Change::Unchanged,
        })
    );
    assert_eq!(
        topic.sample_comparisons.get("sample-b"),
        Some(&SampleComparison {
            sample_id: "sample-b".to_owned(),
            delta: 0.0,
            direction: Change::Incomparable,
        })
    );
}
