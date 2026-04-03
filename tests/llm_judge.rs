#![cfg(feature = "llm-judge")]

use evalkit::{
    LlmJudgeConfig, LlmJudgeScoreExtractor, Run, Sample, Score, ScoreDefinition, Scorer, llm_judge,
};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

#[tokio::test(flavor = "current_thread")]
async fn llm_judge_sends_prompt_and_parses_score_json() {
    let (base_url, requests) = spawn_mock_server(
        200,
        r#"{"choices":[{"message":{"content":"{\"type\":\"numeric\",\"value\":0.75}"}}]}"#,
    );
    let scorer = llm_judge(LlmJudgeConfig {
        model: String::from("judge-mini"),
        base_url,
        prompt_template: String::from(
            "Input: {{input}}\nOutput: {{output}}\nReference: {{reference}}",
        ),
        score_extractor: LlmJudgeScoreExtractor::JsonScore,
        api_key: String::from("secret-key"),
    });
    let input = String::from("What is the capital of France?");
    let output = String::from("Paris");
    let reference = String::from("Paris");
    let definition = scorer.definition();

    let score = run_single_sample(Sample::new(input.clone(), reference), output, scorer).await;

    assert_eq!(score, Score::Numeric(0.75));
    assert_eq!(definition, ScoreDefinition::new("llm_judge"));

    let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    let prompt = request.body["messages"][0]["content"].as_str().unwrap();

    assert_eq!(request.authorization.as_deref(), Some("Bearer secret-key"));
    assert_eq!(request.body["model"].as_str(), Some("judge-mini"));
    assert!(prompt.contains("Input: What is the capital of France?"));
    assert!(prompt.contains("Output: Paris"));
    assert!(prompt.contains("Reference: Paris"));
}

#[tokio::test(flavor = "current_thread")]
async fn llm_judge_supports_boolean_extraction_without_reference() {
    let (base_url, requests) =
        spawn_mock_server(200, r#"{"choices":[{"message":{"content":"true"}}]}"#);
    let scorer = llm_judge(LlmJudgeConfig {
        model: String::from("judge-bool"),
        base_url,
        prompt_template: String::from("Input={{input}} Output={{output}} Reference={{reference}}"),
        score_extractor: LlmJudgeScoreExtractor::Boolean,
        api_key: String::from("secret-key"),
    });
    let input = String::from("prompt");
    let output = String::from("candidate answer");

    let sample = Sample::builder(input.clone()).id("sample-no-ref").build();
    let score = run_single_sample(sample, output, scorer).await;

    assert_eq!(score, Score::Binary(true));

    let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    let prompt = request.body["messages"][0]["content"].as_str().unwrap();

    assert!(prompt.contains("Input=prompt"));
    assert!(prompt.contains("Output=candidate answer"));
    assert!(prompt.contains("Reference="));
}

#[tokio::test(flavor = "current_thread")]
async fn llm_judge_parse_errors_return_scorer_error() {
    let (base_url, _requests) =
        spawn_mock_server(200, r#"{"choices":[{"message":{"content":"not-a-bool"}}]}"#);
    let scorer = llm_judge(LlmJudgeConfig {
        model: String::from("judge-bool"),
        base_url,
        prompt_template: String::from("{{output}}"),
        score_extractor: LlmJudgeScoreExtractor::Boolean,
        api_key: String::from("secret-key"),
    });
    let input = String::from("prompt");
    let output = String::from("candidate answer");
    let reference = String::from("reference answer");

    let err = run_single_sample_error(Sample::new(input.clone(), reference), output, scorer).await;

    assert!(
        err.to_string()
            .starts_with("llm_judge scorer could not parse model `judge-bool` output:")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn llm_judge_network_errors_return_scorer_error() {
    let base_url = unused_base_url();
    let scorer = llm_judge(LlmJudgeConfig {
        model: String::from("judge-offline"),
        base_url,
        prompt_template: String::from("{{output}}"),
        score_extractor: LlmJudgeScoreExtractor::Numeric,
        api_key: String::from("secret-key"),
    });
    let input = String::from("prompt");
    let output = String::from("candidate answer");
    let reference = String::from("reference answer");

    let err = run_single_sample_error(Sample::new(input.clone(), reference), output, scorer).await;

    assert!(
        err.to_string()
            .starts_with("llm_judge scorer network failure for model `judge-offline`:")
    );
}

#[test]
fn llm_judge_config_skips_api_key_during_serde() {
    let config = LlmJudgeConfig {
        model: String::from("judge-mini"),
        base_url: String::from("https://example.invalid/v1"),
        prompt_template: String::from("{{output}}"),
        score_extractor: LlmJudgeScoreExtractor::Label,
        api_key: String::from("secret-key"),
    };

    let serialized = serde_json::to_value(&config).unwrap();
    let round_tripped: LlmJudgeConfig = serde_json::from_value(serialized.clone()).unwrap();

    assert_eq!(serialized["model"], "judge-mini");
    assert!(serialized.get("api_key").is_none());
    assert_eq!(round_tripped.api_key, "");
}

struct CapturedRequest {
    authorization: Option<String>,
    body: Value,
}

fn spawn_mock_server(
    status_code: u16,
    response_body: &'static str,
) -> (String, Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let raw_request = read_http_request(&mut stream);
        let request = parse_request(&raw_request);
        sender.send(request).unwrap();

        let response = format!(
            "HTTP/1.1 {status_code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    (format!("http://{address}/v1"), receiver)
}

fn read_http_request(stream: &mut impl Read) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let bytes_read = stream.read(&mut chunk).unwrap();
        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);

        if header_end.is_none()
            && let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n")
        {
            let end = position + 4;
            header_end = Some(end);
            let headers = String::from_utf8_lossy(&buffer[..end]);
            content_length = parse_content_length(&headers);
        }

        if let Some(end) = header_end
            && buffer.len() >= end + content_length
        {
            break;
        }
    }

    buffer
}

fn parse_content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn parse_request(raw_request: &[u8]) -> CapturedRequest {
    let header_end = raw_request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap()
        + 4;
    let headers = String::from_utf8(raw_request[..header_end].to_vec()).unwrap();
    let body = serde_json::from_slice::<Value>(&raw_request[header_end..]).unwrap();
    let authorization = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("authorization")
            .then(|| value.trim().to_string())
    });

    CapturedRequest {
        authorization,
        body,
    }
}

fn unused_base_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}/v1")
}

async fn run_single_sample(
    sample: Sample<String, String>,
    output: String,
    scorer: impl evalkit::Scorer<String, String, String> + 'static,
) -> Score {
    let result = execute_single_sample(sample, output, scorer).await;

    result.samples[0].trials[0]
        .scores
        .get("llm_judge")
        .unwrap()
        .as_ref()
        .unwrap()
        .clone()
}

async fn run_single_sample_error(
    sample: Sample<String, String>,
    output: String,
    scorer: impl evalkit::Scorer<String, String, String> + 'static,
) -> String {
    let result = execute_single_sample(sample, output, scorer).await;

    result.samples[0].trials[0]
        .scores
        .get("llm_judge")
        .unwrap()
        .as_ref()
        .unwrap_err()
        .to_string()
}

async fn execute_single_sample(
    sample: Sample<String, String>,
    output: String,
    scorer: impl evalkit::Scorer<String, String, String> + 'static,
) -> evalkit::RunResult {
    let run = Run::builder()
        .dataset(vec![sample])
        .acquisition(move |_input: &String| {
            let output = output.clone();
            async move { Ok(output) }
        })
        .scorer(scorer)
        .build()
        .unwrap();

    run.execute().await.unwrap()
}
