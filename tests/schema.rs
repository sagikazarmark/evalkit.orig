use evalkit::{RUN_RESULT_SCHEMA_VERSION, schema};
use serde_json::json;

#[test]
fn run_log_schema_matches_the_published_document() {
    let published: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/run-log-v1.schema.json"))
            .expect("published schema must be valid JSON");

    assert_eq!(schema::run_log_schema(), published);
}

#[test]
fn run_log_schema_uses_the_current_schema_version() {
    let schema = schema::run_log_schema();

    assert_eq!(
        schema["$defs"]["HeaderRecord"]["properties"]["schema_version"]["const"],
        json!(RUN_RESULT_SCHEMA_VERSION)
    );
    assert_eq!(schema["$id"], json!("https://evalkit.dev/schema/run-log-v1.schema.json"));
}

#[test]
fn run_log_schema_exposes_run_result_and_jsonl_record_shapes() {
    let schema = schema::run_log_schema();

    assert_eq!(schema["oneOf"].as_array().map(Vec::len), Some(3));
    assert!(schema["$defs"].get("RunResult").is_some());
    assert!(schema["$defs"].get("SampleResult").is_some());
    assert!(schema["$defs"].get("TrialResult").is_some());
    assert!(schema["$defs"].get("Score").is_some());
    assert!(schema["$defs"].get("RunMetadata").is_some());
}
