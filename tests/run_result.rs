use std::collections::HashMap;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use evalkit::{
    Direction, RunMetadata, RunResult, SampleResult, Score, ScoreDefinition, ScorerError,
    TokenUsage,
    TrialResult,
};
use serde_json::json;

fn metadata() -> RunMetadata {
    RunMetadata {
        run_id: "run-123".to_owned(),
        seed: Some(7),
        dataset_fingerprint: "dataset-abc".to_owned(),
        scorer_fingerprint: "scorers-abc".to_owned(),
        started_at: Utc.with_ymd_and_hms(2026, 4, 3, 10, 0, 0).unwrap(),
        completed_at: Utc.with_ymd_and_hms(2026, 4, 3, 10, 0, 5).unwrap(),
        duration: Duration::from_secs(5),
        trial_count: 2,
        score_definitions: vec![ScoreDefinition {
            name: "accuracy".to_owned(),
            direction: Some(Direction::Maximize),
        }],
        acquisition_mode: "inline".to_owned(),
    }
}

#[test]
fn trial_result_serializes_scores_and_errors_distinctly() {
    let trial = TrialResult {
        scores: HashMap::from([
            ("zeta".to_owned(), Ok(Score::Binary(false))),
            (
                "alpha".to_owned(),
                Err(ScorerError::internal(std::io::Error::other("boom"))),
            ),
        ]),
        duration: Duration::from_millis(25),
        trial_index: 0,
    };

    let encoded = serde_json::to_string(&trial).expect("trial should serialize");
    let value = serde_json::from_str::<serde_json::Value>(&encoded).expect("json should parse");

    assert!(encoded.find("\"alpha\"").unwrap() < encoded.find("\"zeta\"").unwrap());
    assert_eq!(
        value["scores"]["zeta"],
        json!({ "Ok": { "type": "binary", "value": false } })
    );
    assert_eq!(value["scores"]["alpha"], json!({ "Err": "boom" }));
}

#[test]
fn trial_result_deserializes_error_entries_back_into_scorer_errors() {
    let value = json!({
        "scores": {
            "accuracy": { "Ok": { "type": "numeric", "value": 0.75 } },
            "parser": { "Err": "invalid json" }
        },
        "duration": { "secs": 0, "nanos": 42_000_000 },
        "trial_index": 1
    });

    let trial: TrialResult = serde_json::from_value(value).expect("trial should deserialize");

    match trial
        .scores
        .get("accuracy")
        .expect("accuracy score present")
    {
        Ok(Score::Numeric(value)) => assert_eq!(*value, 0.75),
        other => panic!("unexpected accuracy result: {other:?}"),
    }

    let parser_error = trial
        .scores
        .get("parser")
        .expect("parser score present")
        .as_ref()
        .expect_err("parser should deserialize as an error");

    assert_eq!(parser_error.to_string(), "invalid json");
    assert_eq!(trial.duration, Duration::from_millis(42));
    assert_eq!(trial.trial_index, 1);
}

#[test]
fn sample_result_can_distinguish_low_scores_from_failed_scores() {
    let sample = SampleResult {
        sample_id: "sample-1".to_owned(),
        trials: vec![
            TrialResult {
                scores: HashMap::from([("accuracy".to_owned(), Ok(Score::Binary(false)))]),
                duration: Duration::from_millis(10),
                trial_index: 0,
            },
            TrialResult {
                scores: HashMap::from([(
                    "accuracy".to_owned(),
                    Err(ScorerError::internal(std::io::Error::other("timeout"))),
                )]),
                duration: Duration::from_millis(11),
                trial_index: 1,
            },
        ],
        trial_count: 2,
        scored_count: 1,
        error_count: 1,
        token_usage: TokenUsage {
            input: 20,
            output: 10,
            cache_read: 3,
            cache_write: 1,
        },
        cost_usd: Some(0.01),
    };

    assert!(matches!(
        sample.trials[0].scores.get("accuracy"),
        Some(Ok(Score::Binary(false)))
    ));
    assert!(matches!(
        sample.trials[1].scores.get("accuracy"),
        Some(Err(_))
    ));
    assert_eq!(sample.scored_count, 1);
    assert_eq!(sample.error_count, 1);
    assert_eq!(sample.token_usage.input, 20);
    assert_eq!(sample.cost_usd, Some(0.01));
}

#[test]
fn run_result_round_trips_metadata_and_sample_order() {
    let run = RunResult {
        metadata: metadata(),
        samples: vec![
            SampleResult {
                sample_id: "sample-a".to_owned(),
                trials: vec![TrialResult {
                    scores: HashMap::from([("accuracy".to_owned(), Ok(Score::Binary(true)))]),
                    duration: Duration::from_millis(5),
                    trial_index: 0,
                }],
                trial_count: 1,
                scored_count: 1,
                error_count: 0,
                token_usage: Default::default(),
                cost_usd: None,
            },
            SampleResult {
                sample_id: "sample-b".to_owned(),
                trials: vec![TrialResult {
                    scores: HashMap::from([(
                        "accuracy".to_owned(),
                        Err(ScorerError::internal(std::io::Error::other("bad output"))),
                    )]),
                    duration: Duration::from_millis(7),
                    trial_index: 0,
                }],
                trial_count: 1,
                scored_count: 0,
                error_count: 1,
                token_usage: TokenUsage {
                    input: 7,
                    output: 4,
                    cache_read: 0,
                    cache_write: 0,
                },
                cost_usd: Some(0.002),
            },
        ],
    };

    let decoded: RunResult =
        serde_json::from_str(&serde_json::to_string(&run).expect("run should serialize"))
            .expect("run should deserialize");

    assert_eq!(decoded.metadata.run_id, "run-123");
    assert_eq!(decoded.metadata.seed, Some(7));
    assert_eq!(decoded.metadata.dataset_fingerprint, "dataset-abc");
    assert_eq!(decoded.metadata.scorer_fingerprint, "scorers-abc");
    assert_eq!(decoded.metadata.score_definitions.len(), 1);
    assert_eq!(decoded.samples.len(), 2);
    assert_eq!(decoded.samples[0].sample_id, "sample-a");
    assert_eq!(decoded.samples[1].sample_id, "sample-b");
    assert_eq!(decoded.samples[1].token_usage.output, 4);
    assert_eq!(decoded.samples[1].cost_usd, Some(0.002));
    assert!(matches!(
        decoded.samples[1].trials[0].scores.get("accuracy"),
        Some(Err(error)) if error.to_string() == "bad output"
    ));
}
