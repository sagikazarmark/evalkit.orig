use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::process::Command;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use evalkit::{
    Comparison, RunMetadata, RunResult, SampleResult, Score, ScoreDefinition, TrialResult,
    read_jsonl, write_jsonl,
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
            source_mode: String::from("inline"),
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

#[test]
fn watch_command_runs_initial_eval_when_max_runs_is_one() {
    let temp = tempdir().unwrap();
    let dataset_path = temp.path().join("dataset.jsonl");
    let config_path = temp.path().join("eval.toml");
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let plugin_path = repo_root.join("python/evalkit_plugin/examples/echo_acquisition.py");

    std::fs::write(
        &dataset_path,
        "{\"input\":\"hello\",\"reference\":\"echo::hello\"}\n",
    )
    .unwrap();
    std::fs::write(
        &config_path,
        format!(
            "[acquisition]\ncommand = [\"python3\", \"{}\"]\n\n[[scorer]]\ntype = \"exact_match\"\n",
            plugin_path.display()
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_evalkit"))
        .arg("watch")
        .arg("--dataset")
        .arg(&dataset_path)
        .arg("--config")
        .arg(&config_path)
        .arg("--max-runs")
        .arg("1")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Running eval..."));
}

#[test]
fn run_command_filters_dataset_by_split_tags_and_metadata() {
    let temp = tempdir().unwrap();
    let dataset_path = temp.path().join("dataset.jsonl");
    let config_path = temp.path().join("eval.toml");
    let output_path = temp.path().join("result.jsonl");
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let plugin_path = repo_root.join("python/evalkit_plugin/examples/echo_acquisition.py");

    std::fs::write(
        &dataset_path,
        concat!(
            "{\"id\":\"keep\",\"input\":\"hello\",\"reference\":\"echo::hello\",\"split\":\"validation\",\"tags\":[\"smoke\",\"en\"],\"metadata\":{\"locale\":\"en\"}}\n",
            "{\"id\":\"drop\",\"input\":\"bonjour\",\"reference\":\"echo::bonjour\",\"split\":\"validation\",\"tags\":[\"smoke\"],\"metadata\":{\"locale\":\"fr\"}}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        &config_path,
        format!(
            concat!(
                "[acquisition]\n",
                "command = [\"python3\", \"{}\"]\n\n",
                "[dataset]\n",
                "split = \"validation\"\n",
                "tags = [\"smoke\", \"en\"]\n",
                "metadata = {{ locale = \"en\" }}\n\n",
                "[[scorer]]\n",
                "type = \"exact_match\"\n"
            ),
            plugin_path.display()
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_evalkit"))
        .arg("run")
        .arg("--dataset")
        .arg(&dataset_path)
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(output.status.success());

    let result = read_jsonl(BufReader::new(File::open(&output_path).unwrap())).unwrap();
    assert_eq!(result.samples.len(), 1);
    assert_eq!(result.samples[0].sample_id, "keep");
}
