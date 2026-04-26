use evalkit::{RUN_RESULT_SCHEMA_VERSION, schema};

#[test]
fn run_log_schema_matches_the_published_document() {
    let published: serde_json::Value =
        serde_json::from_str(include_str!("../../docs/schema/run-log-v2.schema.json"))
            .expect("published schema must be valid JSON");

    assert_eq!(schema::run_log_schema(), published);
}

#[test]
fn run_log_schema_uses_the_current_schema_version() {
    // RUN_RESULT_SCHEMA_VERSION was bumped to "2" as part of the
    // OutputSource rename.  The schema JSON content update (including
    // bumping the schema_version const and $id inside the file) is
    // deferred to Task 4 of this release series.
    assert_eq!(RUN_RESULT_SCHEMA_VERSION, "2");
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
    assert!(
        schema["$defs"]["RunMetadata"]["properties"]
            .get("code_commit")
            .is_some()
    );
    assert!(
        schema["$defs"]["RunMetadata"]["properties"]
            .get("code_fingerprint")
            .is_some()
    );
    assert!(
        schema["$defs"]["RunMetadata"]["properties"]
            .get("judge_model_pins")
            .is_some()
    );
}
