use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::process::Command;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use evalkit::{
    read_jsonl, write_jsonl, Comparison, RunMetadata, RunResult, SampleResult, Score,
    ScoreDefinition, TrialResult,
};
use tempfile::tempdir;

fn fixture_run(run_id: &str, accuracy: f64) -> RunResult {
    RunResult {
        metadata: RunMetadata {
            run_id: run_id.to_owned(),
            seed: None,
            dataset_fingerprint: format!("dataset-{run_id}"),
            scorer_fingerprint: format!("scorers-{run_id}"),
            code_commit: None,
            code_fingerprint: None,
            judge_model_pins: Vec::new(),
            started_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
            completed_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 5).unwrap(),
            duration: Duration::from_secs(5),
            trial_count: 1,
            score_definitions: vec![ScoreDefinition::maximize("accuracy")],
            acquisition_mode: String::from("inline"),
        },
        samples: vec![SampleResult {
            sample_id: String::from("sample-1"),
            trial_count: 1,
            scored_count: 1,
            error_count: 0,
            token_usage: Default::default(),
            cost_usd: None,
            trials: vec![TrialResult {
                scores: [(String::from("accuracy"), Ok(Score::Numeric(accuracy)))]
                    .into_iter()
                    .collect(),
                duration: Duration::from_millis(10),
                trial_index: 0,
            }],
        }],
    }
}

#[test]
fn diff_command_writes_markdown_and_json_outputs() {
    let temp = tempdir().unwrap();
    let baseline_path = temp.path().join("baseline.jsonl");
    let candidate_path = temp.path().join("candidate.jsonl");
    let markdown_path = temp.path().join("diff.md");
    let json_path = temp.path().join("diff.json");

    write_jsonl(
        &fixture_run("baseline", 0.4),
        BufWriter::new(File::create(&baseline_path).unwrap()),
    )
    .unwrap();
    write_jsonl(
        &fixture_run("candidate", 0.8),
        BufWriter::new(File::create(&candidate_path).unwrap()),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_evalkit"))
        .arg("diff")
        .arg(&baseline_path)
        .arg(&candidate_path)
        .arg("--output")
        .arg(&markdown_path)
        .arg("--json-output")
        .arg(&json_path)
        .output()
        .unwrap();

    assert!(output.status.success());

    let markdown = std::fs::read_to_string(&markdown_path).unwrap();
    assert!(markdown.contains("## Eval Diff"));
    assert!(markdown.contains("`accuracy`"));
    assert!(markdown.contains("Baseline: `baseline`"));
    assert!(markdown.contains("Candidate: `candidate`"));

    let json: Comparison =
        serde_json::from_reader(BufReader::new(File::open(&json_path).unwrap())).unwrap();
    assert_eq!(json.baseline_id, "baseline");
    assert_eq!(json.candidate_id, "candidate");
    assert!(json.shared_scorers.contains_key("accuracy"));

    let baseline_round_trip =
        read_jsonl(BufReader::new(File::open(&baseline_path).unwrap())).unwrap();
    assert_eq!(baseline_round_trip.metadata.run_id, "baseline");
}
