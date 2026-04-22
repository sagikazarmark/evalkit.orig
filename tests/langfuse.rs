#![cfg(feature = "langfuse")]

use evalkit::{LangfuseConfig, RunMetadata, RunResult, SampleResult, Score, ScoreDefinition, TrialResult, export_run};
use chrono::Utc;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn make_result() -> RunResult {
    let now = Utc::now();
    RunResult {
        metadata: RunMetadata {
            run_id: "run-test".into(),
            seed: None,
            dataset_fingerprint: "dataset-langfuse".into(),
            scorer_fingerprint: "scorers-langfuse".into(),
            started_at: now,
            completed_at: now,
            duration: Duration::from_secs(1),
            trial_count: 1,
            score_definitions: vec![ScoreDefinition::new("exact_match")],
            acquisition_mode: "inline".into(),
        },
        samples: vec![SampleResult {
            sample_id: "sample-1".into(),
            trial_count: 1,
            scored_count: 1,
            error_count: 0,
            token_usage: Default::default(),
            cost_usd: None,
            trials: vec![TrialResult {
                scores: HashMap::from([("exact_match".into(), Ok(Score::Binary(true)))]),
                duration: Duration::from_millis(5),
                trial_index: 0,
            }],
        }],
    }
}

fn http_ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

fn spawn_server(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);

    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 16384];
            let n = stream.read(&mut buf).unwrap();
            recorded.lock().unwrap().push(String::from_utf8_lossy(&buf[..n]).into_owned());
            stream.write_all(response.as_bytes()).unwrap();
        }
    });

    (address, requests, handle)
}

#[tokio::test(flavor = "current_thread")]
async fn export_run_posts_batch_to_ingestion_endpoint() {
    let result = make_result();
    let (base_url, requests, server) = spawn_server(vec![http_ok(r#"{"successes":[],"errors":[]}"#)]);

    let config = LangfuseConfig {
        host: base_url,
        public_key: "pk-test".into(),
        secret_key: "sk-test".into(),
    };

    export_run(&result, &config).await.unwrap();
    server.join().unwrap();

    let raw = requests.lock().unwrap()[0].clone();
    let raw_lower = raw.to_ascii_lowercase();
    assert!(raw.contains("POST /api/public/ingestion"));
    assert!(raw_lower.contains("authorization: basic"));
    assert!(raw.contains("trace-create"));
    assert!(raw.contains("score-create"));
    assert!(raw.contains("exact_match"));
    assert!(raw.contains("run-test/sample-1"));
}

#[tokio::test(flavor = "current_thread")]
async fn export_run_returns_error_on_non_2xx_response() {
    let result = make_result();
    let (base_url, _, server) = spawn_server(vec![
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
    ]);

    let config = LangfuseConfig {
        host: base_url,
        public_key: "bad-key".into(),
        secret_key: "bad-secret".into(),
    };

    let err = export_run(&result, &config).await.unwrap_err();
    server.join().unwrap();
    assert!(err.to_string().contains("Langfuse export failed"));
}
