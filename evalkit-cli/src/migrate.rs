use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use serde_json::Value;

/// Migrate a run-log JSONL file from schema v2 to v3.
///
/// Reads each line from `input`, transforms it according to the v2 → v3
/// shape changes (Score::Structured → Numeric+reasoning, add source_metadata
/// and source_resources/scorer_resources defaults), and writes to `output`.
pub fn migrate_v2_to_v3(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);
    let mut writer = BufWriter::new(File::create(output)?);

    let mut first_line = true;
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() { continue; }
        let mut value: Value = serde_json::from_str(&line)?;

        if first_line {
            // Header line — bump schema_version
            if let Some(header) = value.get_mut("Header") {
                header["schema_version"] = Value::String("3".into());
            }
            // Also handle flat header (record_type / schema_version at top level)
            if value.get("schema_version").is_some() {
                value["schema_version"] = Value::String("3".into());
            }
            first_line = false;
        } else if value.get("RunResult").is_some() {
            // Standalone RunResult body
            if let Some(run_result) = value.get_mut("RunResult") {
                transform_run_result(run_result);
            }
        } else if value.get("samples").is_some() {
            // Direct RunResult JSON (no record-type tag — older variant)
            transform_run_result(&mut value);
        } else if value.get("Sample").is_some() {
            // Per-sample JSONL record (if the format supports streamed samples)
            if let Some(sample) = value.get_mut("Sample").and_then(|s| s.get_mut("sample")) {
                transform_sample(sample);
            }
        }

        writeln!(writer, "{}", serde_json::to_string(&value)?)?;
    }
    writer.flush()?;
    Ok(())
}

/// Transform a RunResult JSON value in place.
pub fn transform_run_result(value: &mut Value) {
    let Some(samples) = value.get_mut("samples").and_then(|v| v.as_array_mut()) else { return; };
    for sample in samples {
        transform_sample(sample);
    }
}

fn transform_sample(sample: &mut Value) {
    if let Some(trials) = sample.get_mut("trials").and_then(|v| v.as_array_mut()) {
        for trial in trials {
            if let Some(scores) = trial.get_mut("scores").and_then(|v| v.as_object_mut()) {
                for (_name, entry) in scores.iter_mut() {
                    transform_score_entry(entry);
                }
            }
            if let Some(obj) = trial.as_object_mut() {
                obj.entry("source_metadata")
                    .or_insert_with(|| Value::Object(Default::default()));
            }
        }
    }
    if let Some(obj) = sample.as_object_mut() {
        obj.entry("source_resources").or_insert_with(default_resource_usage);
        obj.entry("scorer_resources").or_insert_with(default_resource_usage);
    }
}

fn default_resource_usage() -> Value {
    serde_json::json!({
        "token_usage": {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0},
        "cost_usd": null,
        "latency": null
    })
}

/// Transform a single score entry from v2 (`Result<Score, ScorerError>`-shaped)
/// to v3 (`ScoredEntry { result, reasoning, metadata }`).
pub fn transform_score_entry(entry: &mut Value) {
    // v2 entry was either {"Ok": Score} or {"Err": String}.
    // v3 wraps that as ScoredEntry { result: <v2-shape>, reasoning, metadata }.

    // First: handle Score::Structured → Numeric + reasoning + metadata.
    if let Some(ok) = entry.get("Ok") {
        if let Some("structured") = ok.get("type").and_then(|t| t.as_str()) {
            let score_val = ok.get("score").cloned().unwrap_or(Value::Null);
            let reasoning = ok.get("reasoning").cloned().unwrap_or(Value::Null);
            let metadata = ok.get("metadata").cloned().unwrap_or_else(|| Value::Object(Default::default()));
            *entry = serde_json::json!({
                "result": { "Ok": { "type": "numeric", "value": score_val } },
                "reasoning": reasoning,
                "metadata": metadata,
            });
            return;
        }
    }

    // Otherwise: wrap the existing Result<Score,_> shape into ScoredEntry.
    let old_result = entry.clone();
    *entry = serde_json::json!({
        "result": old_result,
        "reasoning": null,
        "metadata": {},
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_score_migrates_to_numeric_with_reasoning() {
        let mut entry = serde_json::json!({
            "Ok": { "type": "structured", "score": 0.75, "reasoning": "good", "metadata": {"k": "v"} }
        });
        transform_score_entry(&mut entry);
        assert_eq!(entry["result"]["Ok"]["type"], "numeric");
        assert_eq!(entry["result"]["Ok"]["value"], 0.75);
        assert_eq!(entry["reasoning"], "good");
        assert_eq!(entry["metadata"]["k"], "v");
    }

    #[test]
    fn binary_score_wraps_with_null_reasoning() {
        let mut entry = serde_json::json!({
            "Ok": { "type": "binary", "value": true }
        });
        transform_score_entry(&mut entry);
        assert_eq!(entry["result"]["Ok"]["type"], "binary");
        assert_eq!(entry["result"]["Ok"]["value"], true);
        assert_eq!(entry["reasoning"], Value::Null);
        assert!(entry["metadata"].is_object() && entry["metadata"].as_object().unwrap().is_empty());
    }

    #[test]
    fn err_entry_wraps_with_null_reasoning() {
        let mut entry = serde_json::json!({ "Err": "scorer failed" });
        transform_score_entry(&mut entry);
        assert_eq!(entry["result"]["Err"], "scorer failed");
        assert_eq!(entry["reasoning"], Value::Null);
    }
}
