use std::path::PathBuf;
use std::time::Duration;

use evalkit::Score;
use evalkit_providers::{
    PluginKind, ScorerPluginRequest, conformance_check_source_plugin,
    conformance_check_scorer_plugin,
};
use serde_json::json;

#[tokio::test(flavor = "current_thread")]
async fn python_source_shim_passes_conformance() {
    let report = conformance_check_source_plugin(
        "python3",
        vec![python_example("echo_source.py")],
        "prompt",
        Duration::from_secs(5),
    )
    .await
    .unwrap();

    assert_eq!(report.handshake.kind, PluginKind::Source);
    assert_eq!(report.handshake.name, "echo-source");
    assert_eq!(report.output, "echo::prompt");
}

#[tokio::test(flavor = "current_thread")]
async fn python_scorer_shim_passes_conformance() {
    let report = conformance_check_scorer_plugin(
        "python3",
        vec![python_example("exact_match_scorer.py")],
        ScorerPluginRequest {
            input: String::from("prompt"),
            output: String::from("answer"),
            reference: Some(String::from("answer")),
            run_id: Some(String::from("run-1")),
            sample_id: Some(String::from("sample-1")),
            trial_index: 0,
            metadata: json!({"topic":"math"}),
        },
        Duration::from_secs(5),
    )
    .await
    .unwrap();

    assert_eq!(report.handshake.kind, PluginKind::Scorer);
    assert_eq!(report.handshake.name, "exact-match-scorer");
    assert_eq!(report.score, Score::Binary(true));
}

fn python_example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("python")
        .join("evalkit_plugin")
        .join("examples")
        .join(name);

    path.to_string_lossy().into_owned()
}
