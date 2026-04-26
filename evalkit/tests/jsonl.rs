use std::collections::HashMap;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use evalkit::{
    Direction, RUN_RESULT_SCHEMA_VERSION, RunMetadata, RunResult, SampleResult, Score,
    ScoreDefinition, ScorerError, TrialResult, read_jsonl, write_jsonl,
};
use serde_json::json;

fn run_result() -> RunResult {
    RunResult {
        metadata: RunMetadata {
            run_id: "run-456".to_owned(),
            seed: Some(11),
            dataset_fingerprint: "dataset-jsonl".to_owned(),
            scorer_fingerprint: "scorers-jsonl".to_owned(),
            code_commit: Some("deadbeef".to_owned()),
            code_fingerprint: Some("tree:1234abcd".to_owned()),
            judge_model_pins: vec!["gpt-4o@2026-04-01".to_owned()],
            started_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
            completed_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 5).unwrap(),
            duration: Duration::from_secs(5),
            trial_count: 2,
            score_definitions: vec![ScoreDefinition {
                name: "latency".to_owned(),
                direction: Some(Direction::Minimize),
            }],
            acquisition_mode: "inline".to_owned(),
        },
        samples: vec![
            SampleResult {
                sample_id: "sample-a".to_owned(),
                trials: vec![TrialResult {
                    scores: HashMap::from([
                        (
                            "latency".to_owned(),
                            Ok(Score::Metric {
                                name: "latency_ms".to_owned(),
                                value: 120.0,
                                unit: Some("ms".to_owned()),
                            }),
                        ),
                        (
                            "parser".to_owned(),
                            Err(ScorerError::internal(std::io::Error::other("bad trace"))),
                        ),
                    ]),
                    duration: Duration::from_millis(10),
                    trial_index: 0,
                }],
                trial_count: 1,
                scored_count: 1,
                error_count: 1,
                token_usage: evalkit::TokenUsage {
                    input: 12,
                    output: 6,
                    cache_read: 0,
                    cache_write: 0,
                },
                cost_usd: Some(0.0025),
            },
            SampleResult {
                sample_id: "sample-b".to_owned(),
                trials: vec![TrialResult {
                    scores: HashMap::from([("latency".to_owned(), Ok(Score::Numeric(98.0)))]),
                    duration: Duration::from_millis(8),
                    trial_index: 0,
                }],
                trial_count: 1,
                scored_count: 1,
                error_count: 0,
                token_usage: Default::default(),
                cost_usd: None,
            },
        ],
    }
}

#[test]
fn write_jsonl_serializes_metadata_then_samples_as_jsonl() {
    let result = run_result();
    let mut buffer = Vec::new();

    write_jsonl(&result, &mut buffer).expect("jsonl should serialize");

    let encoded = String::from_utf8(buffer).expect("writer should contain utf-8 json");
    let lines: Vec<_> = encoded.lines().collect();

    assert_eq!(lines.len(), 4);

    let header = serde_json::from_str::<serde_json::Value>(lines[0]).expect("header json");
    let metadata = serde_json::from_str::<serde_json::Value>(lines[1]).expect("metadata json");
    let sample_a = serde_json::from_str::<serde_json::Value>(lines[2]).expect("sample a json");
    let sample_b = serde_json::from_str::<serde_json::Value>(lines[3]).expect("sample b json");

    assert_eq!(header["record_type"], json!("header"));
    assert_eq!(header["schema_version"], json!(RUN_RESULT_SCHEMA_VERSION));
    assert_eq!(metadata["record_type"], json!("metadata"));
    assert_eq!(metadata["metadata"]["run_id"], json!("run-456"));
    assert_eq!(metadata["metadata"]["code_commit"], json!("deadbeef"));
    assert_eq!(
        metadata["metadata"]["judge_model_pins"],
        json!(["gpt-4o@2026-04-01"])
    );
    assert_eq!(sample_a["record_type"], json!("sample"));
    assert_eq!(sample_a["sample"]["sample_id"], json!("sample-a"));
    assert_eq!(sample_a["sample"]["token_usage"]["input"], json!(12));
    assert_eq!(sample_a["sample"]["cost_usd"], json!(0.0025));
    assert_eq!(sample_b["sample"]["sample_id"], json!("sample-b"));
    assert!(
        lines[2].find("\"latency\"").unwrap() < lines[2].find("\"parser\"").unwrap(),
        "existing sorted scorer ordering should be preserved inside each sample line"
    );
}

#[test]
fn read_jsonl_round_trips_back_to_a_typed_run_result() {
    let expected = run_result();
    let mut buffer = Vec::new();

    write_jsonl(&expected, &mut buffer).expect("jsonl should serialize");

    let decoded = read_jsonl(buffer.as_slice()).expect("jsonl should deserialize");

    assert_eq!(decoded.metadata.run_id, expected.metadata.run_id);
    assert_eq!(decoded.metadata.score_definitions.len(), 1);
    assert_eq!(decoded.samples.len(), 2);
    assert_eq!(decoded.metadata.code_commit.as_deref(), Some("deadbeef"));
    assert_eq!(decoded.metadata.judge_model_pins.len(), 1);
    assert_eq!(decoded.samples[0].sample_id, "sample-a");
    assert_eq!(decoded.samples[1].sample_id, "sample-b");
    assert_eq!(decoded.samples[0].token_usage.input, 12);
    assert_eq!(decoded.samples[0].cost_usd, Some(0.0025));
    assert!(matches!(
        decoded.samples[0].trials[0].scores.get("latency"),
        Some(Ok(Score::Metric { name, value, unit }))
            if name == "latency_ms" && *value == 120.0 && unit.as_deref() == Some("ms")
    ));
    assert!(matches!(
        decoded.samples[0].trials[0].scores.get("parser"),
        Some(Err(error)) if error.to_string() == "bad trace"
    ));
}

#[test]
fn read_jsonl_supports_legacy_metadata_first_files() {
    let legacy = concat!(
        "{\"record_type\":\"metadata\",\"metadata\":{\"run_id\":\"run-legacy\",\"seed\":null,\"dataset_fingerprint\":\"dataset-legacy\",\"scorer_fingerprint\":\"scorers-legacy\",\"started_at\":\"2026-04-03T12:00:00Z\",\"completed_at\":\"2026-04-03T12:00:05Z\",\"duration\":{\"secs\":5,\"nanos\":0},\"trial_count\":1,\"score_definitions\":[],\"acquisition_mode\":\"inline\"}}\n",
        "{\"record_type\":\"sample\",\"sample\":{\"sample_id\":\"sample-1\",\"trials\":[],\"trial_count\":0,\"scored_count\":0,\"error_count\":0,\"token_usage\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0},\"cost_usd\":null}}\n"
    );

    let decoded = read_jsonl(legacy.as_bytes()).expect("legacy jsonl should deserialize");

    assert_eq!(decoded.metadata.run_id, "run-legacy");
    assert_eq!(decoded.metadata.code_commit, None);
    assert_eq!(decoded.metadata.code_fingerprint, None);
    assert!(decoded.metadata.judge_model_pins.is_empty());
    assert_eq!(decoded.samples.len(), 1);
}

#[test]
fn read_jsonl_rejects_unknown_schema_versions() {
    let invalid = concat!(
        "{\"record_type\":\"header\",\"schema_version\":\"99\"}\n",
        "{\"record_type\":\"metadata\",\"metadata\":{\"run_id\":\"run-legacy\",\"seed\":null,\"dataset_fingerprint\":\"dataset-legacy\",\"scorer_fingerprint\":\"scorers-legacy\",\"started_at\":\"2026-04-03T12:00:00Z\",\"completed_at\":\"2026-04-03T12:00:05Z\",\"duration\":{\"secs\":5,\"nanos\":0},\"trial_count\":1,\"score_definitions\":[],\"acquisition_mode\":\"inline\"}}\n"
    );

    let err = read_jsonl(invalid.as_bytes()).expect_err("unknown schema version should fail");

    assert!(err.to_string().contains("schema version"));
}
