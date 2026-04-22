use serde_json::Value;

pub const RUN_RESULT_SCHEMA_VERSION: &str = "1";

pub fn run_log_schema() -> Value {
    serde_json::from_str(include_str!("../docs/schema/run-log-v1.schema.json"))
        .expect("run-log schema document must be valid JSON")
}
