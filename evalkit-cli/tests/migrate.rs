use evalkit_cli::migrate::{migrate_v2_to_v3, transform_score_entry};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn migrate_runlog_v2_to_v3_end_to_end() {
    let input_jsonl = concat!(
        r#"{"record_type":"header","schema_version":"2"}"#, "\n",
        r#"{"record_type":"metadata","metadata":{"run_id":"r1","seed":null,"dataset_fingerprint":"d","scorer_fingerprint":"s","started_at":"2026-04-26T00:00:00Z","completed_at":"2026-04-26T00:00:01Z","duration":{"secs":1,"nanos":0},"trial_count":1,"score_definitions":[],"source_mode":"inline"}}"#, "\n",
    );

    let mut input = NamedTempFile::new().unwrap();
    input.write_all(input_jsonl.as_bytes()).unwrap();

    let output = NamedTempFile::new().unwrap();
    migrate_v2_to_v3(input.path(), output.path()).unwrap();

    let result = std::fs::read_to_string(output.path()).unwrap();
    assert!(result.contains("\"schema_version\":\"3\""), "output: {result}");
}

#[test]
fn transform_score_entry_structured_to_numeric() {
    let mut entry = serde_json::json!({
        "Ok": { "type": "structured", "score": 0.9, "reasoning": "excellent", "metadata": {} }
    });
    transform_score_entry(&mut entry);
    assert_eq!(entry["result"]["Ok"]["type"], "numeric");
    assert_eq!(entry["result"]["Ok"]["value"], 0.9);
    assert_eq!(entry["reasoning"], "excellent");
}
