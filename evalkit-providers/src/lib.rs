use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use evalkit::{Acquisition, AcquisitionError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;

pub const PLUGIN_PROTOCOL_VERSION: &str = "1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Acquisition,
    Scorer,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHandshake {
    pub kind: PluginKind,
    pub name: String,
    pub version: String,
    pub schema_version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionPluginRequest {
    pub input: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcquisitionPluginResponse {
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub error: Option<PluginErrorPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquisitionPluginConformance {
    pub handshake: PluginHandshake,
    pub output: String,
}

#[derive(Debug)]
pub struct PluginProtocolError(String);

impl Display for PluginProtocolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for PluginProtocolError {}

#[derive(Debug)]
pub struct PluginReportedError {
    pub payload: PluginErrorPayload,
}

impl Display for PluginReportedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "plugin error [{}]: {}",
            self.payload.code, self.payload.message
        )
    }
}

impl Error for PluginReportedError {}

#[derive(Debug)]
enum PluginResponseError {
    Protocol(PluginProtocolError),
    Reported(PluginReportedError),
}

impl Display for PluginResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => Display::fmt(error, f),
            Self::Reported(error) => Display::fmt(error, f),
        }
    }
}

impl Error for PluginResponseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Reported(error) => Some(error),
        }
    }
}

pub struct HttpAcquisition {
    client: Client,
    url: String,
    input_field: String,
    output_field: String,
}

impl HttpAcquisition {
    pub fn new(
        url: impl Into<String>,
        input_field: impl Into<String>,
        output_field: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder().timeout(timeout).build()?;

        Ok(Self {
            client,
            url: url.into(),
            input_field: input_field.into(),
            output_field: output_field.into(),
        })
    }
}

impl Acquisition<String, String> for HttpAcquisition {
    async fn acquire(&self, input: &String) -> Result<String, AcquisitionError> {
        let body = json!({ &self.input_field: input });
        let response = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

        let payload: Value = response
            .json()
            .await
            .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

        extract_string_field(&payload, &self.output_field)
    }
}

pub struct SubprocessAcquisition {
    program: String,
    args: Vec<String>,
    input_field: String,
    output_field: String,
    timeout: Duration,
}

impl SubprocessAcquisition {
    pub fn new(
        program: impl Into<String>,
        args: Vec<String>,
        input_field: impl Into<String>,
        output_field: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            program: program.into(),
            args,
            input_field: input_field.into(),
            output_field: output_field.into(),
            timeout,
        }
    }

    async fn run(&self, input: &String) -> Result<String, AcquisitionError> {
        let input_json = serde_json::to_string(&json!({ &self.input_field: input }))
            .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

        let mut child = TokioCommand::new(&self.program)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_json.as_bytes())
                .await
                .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;
        }

        let stdout = child.stdout.take().expect("stdout was piped");
        let mut reader = TokioBufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

        if line.trim().is_empty() {
            return Err(AcquisitionError::ExecutionFailed(Box::new(
                EmptyProcessOutput,
            )));
        }

        if let Some(handshake) = parse_plugin_handshake(line.trim()).map_err(protocol_failure)? {
            validate_plugin_handshake(&handshake, PluginKind::Acquisition)
                .map_err(protocol_failure)?;

            line.clear();
            reader
                .read_line(&mut line)
                .await
                .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;
        }

        let _ = child.wait().await;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(AcquisitionError::ExecutionFailed(Box::new(
                EmptyProcessOutput,
            )));
        }

        if let Some(response) = parse_plugin_response(trimmed).map_err(protocol_failure)? {
            extract_plugin_response_output(response).map_err(response_failure_to_acquisition)
        } else {
            let payload: Value = serde_json::from_str(trimmed)
                .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

            extract_string_field(&payload, &self.output_field)
        }
    }
}

impl Acquisition<String, String> for SubprocessAcquisition {
    async fn acquire(&self, input: &String) -> Result<String, AcquisitionError> {
        tokio::time::timeout(self.timeout, self.run(input))
            .await
            .map_err(|_| AcquisitionError::Timeout(self.timeout))?
    }
}

pub async fn conformance_check_acquisition_plugin(
    program: impl Into<String>,
    args: Vec<String>,
    input: impl Into<String>,
    timeout: Duration,
) -> Result<AcquisitionPluginConformance, PluginProtocolError> {
    let input = AcquisitionPluginRequest {
        input: input.into(),
    };
    let input_json = serde_json::to_string(&input).map_err(protocol_error)?;

    let mut child = TokioCommand::new(program.into())
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(protocol_error)?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input_json.as_bytes())
            .await
            .map_err(protocol_error)?;
        stdin.write_all(b"\n").await.map_err(protocol_error)?;
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PluginProtocolError(String::from("plugin stdout was not captured")))?;
    let mut reader = TokioBufReader::new(stdout);
    let mut handshake_line = String::new();

    tokio::time::timeout(timeout, reader.read_line(&mut handshake_line))
        .await
        .map_err(|_| PluginProtocolError(String::from("plugin handshake timed out")))?
        .map_err(protocol_error)?;

    let handshake = parse_plugin_handshake(handshake_line.trim())?
        .ok_or_else(|| PluginProtocolError(String::from("plugin did not emit a handshake line")))?;
    validate_plugin_handshake(&handshake, PluginKind::Acquisition)?;

    let mut response_line = String::new();
    tokio::time::timeout(timeout, reader.read_line(&mut response_line))
        .await
        .map_err(|_| PluginProtocolError(String::from("plugin response timed out")))?
        .map_err(protocol_error)?;

    let _ = child.wait().await;

    let response = parse_plugin_response(response_line.trim())?
        .ok_or_else(|| PluginProtocolError(String::from("plugin did not emit a response line")))?;

    Ok(AcquisitionPluginConformance {
        handshake,
        output: extract_plugin_response_output(response).map_err(response_failure_to_protocol)?,
    })
}

fn parse_plugin_handshake(line: &str) -> Result<Option<PluginHandshake>, PluginProtocolError> {
    let value: Value = serde_json::from_str(line).map_err(protocol_error)?;

    if !looks_like_handshake(&value) {
        return Ok(None);
    }

    serde_json::from_value(value)
        .map(Some)
        .map_err(|source| PluginProtocolError(format!("invalid plugin handshake: {source}")))
}

fn parse_plugin_response(
    line: &str,
) -> Result<Option<AcquisitionPluginResponse>, PluginProtocolError> {
    let value: Value = serde_json::from_str(line).map_err(protocol_error)?;

    if !looks_like_plugin_response(&value) {
        return Ok(None);
    }

    serde_json::from_value(value)
        .map(Some)
        .map_err(|source| PluginProtocolError(format!("invalid plugin response: {source}")))
}

fn looks_like_handshake(value: &Value) -> bool {
    value.get("kind").is_some()
        && value.get("name").is_some()
        && value.get("version").is_some()
        && value.get("schema_version").is_some()
}

fn looks_like_plugin_response(value: &Value) -> bool {
    value.get("output").is_some() || value.get("error").is_some()
}

fn validate_plugin_handshake(
    handshake: &PluginHandshake,
    expected_kind: PluginKind,
) -> Result<(), PluginProtocolError> {
    if handshake.kind != expected_kind {
        return Err(PluginProtocolError(format!(
            "plugin kind mismatch: expected {:?}, got {:?}",
            expected_kind, handshake.kind
        )));
    }

    if handshake.schema_version != PLUGIN_PROTOCOL_VERSION {
        return Err(PluginProtocolError(format!(
            "unsupported plugin schema version `{}`",
            handshake.schema_version
        )));
    }

    if handshake.name.trim().is_empty() {
        return Err(PluginProtocolError(String::from(
            "plugin handshake name must not be empty",
        )));
    }

    if handshake.version.trim().is_empty() {
        return Err(PluginProtocolError(String::from(
            "plugin handshake version must not be empty",
        )));
    }

    Ok(())
}

fn extract_plugin_response_output(
    response: AcquisitionPluginResponse,
) -> Result<String, PluginResponseError> {
    match (response.output, response.error) {
        (Some(output), None) => Ok(output),
        (None, Some(error)) => Err(PluginResponseError::Reported(PluginReportedError {
            payload: error,
        })),
        (Some(_), Some(_)) => Err(PluginResponseError::Protocol(PluginProtocolError(
            String::from("plugin response must not include both `output` and `error`"),
        ))),
        (None, None) => Err(PluginResponseError::Protocol(PluginProtocolError(
            String::from("plugin response must include either `output` or `error`"),
        ))),
    }
}

fn protocol_error(source: impl Error) -> PluginProtocolError {
    PluginProtocolError(source.to_string())
}

fn protocol_failure(error: PluginProtocolError) -> AcquisitionError {
    AcquisitionError::ExecutionFailed(Box::new(error))
}

fn response_failure_to_acquisition(error: PluginResponseError) -> AcquisitionError {
    match error {
        PluginResponseError::Protocol(error) => AcquisitionError::ExecutionFailed(Box::new(error)),
        PluginResponseError::Reported(error) => AcquisitionError::ExecutionFailed(Box::new(error)),
    }
}

fn response_failure_to_protocol(error: PluginResponseError) -> PluginProtocolError {
    PluginProtocolError(error.to_string())
}

fn extract_string_field(payload: &Value, field: &str) -> Result<String, AcquisitionError> {
    match payload.get(field).and_then(Value::as_str) {
        Some(value) => Ok(value.to_owned()),
        None => Err(AcquisitionError::ExecutionFailed(Box::new(
            MissingOutputField(field.to_owned()),
        ))),
    }
}

#[derive(Debug)]
struct MissingOutputField(String);

impl Display for MissingOutputField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "response JSON is missing the `{}` field", self.0)
    }
}

impl Error for MissingOutputField {}

#[derive(Debug)]
struct EmptyProcessOutput;

impl Display for EmptyProcessOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("subprocess produced no output on stdout")
    }
}

impl Error for EmptyProcessOutput {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_string_field_returns_the_requested_value() {
        let payload = json!({ "output": "hello" });

        let output = extract_string_field(&payload, "output").unwrap();

        assert_eq!(output, "hello");
    }

    #[test]
    fn extract_string_field_reports_missing_values() {
        let payload = json!({ "result": "hello" });

        let err = extract_string_field(&payload, "output").unwrap_err();

        assert_eq!(
            err.to_string(),
            "acquisition execution failed: response JSON is missing the `output` field"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn conformance_check_accepts_handshake_and_output() {
        let report = conformance_check_acquisition_plugin(
            "sh",
            vec![
                String::from("-c"),
                String::from(
                    "read line; printf '%s\n' '{\"kind\":\"acquisition\",\"name\":\"demo\",\"version\":\"0.1.0\",\"schema_version\":\"1\",\"capabilities\":[\"structured-errors\"]}' '{\"output\":\"ok\"}'",
                ),
            ],
            "prompt",
            Duration::from_secs(1),
        )
        .await
        .unwrap();

        assert_eq!(report.handshake.kind, PluginKind::Acquisition);
        assert_eq!(report.handshake.name, "demo");
        assert_eq!(report.output, "ok");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn subprocess_acquisition_accepts_handshake_preamble() {
        let acquisition = SubprocessAcquisition::new(
            "sh",
            vec![
                String::from("-c"),
                String::from(
                    "read line; printf '%s\n' '{\"kind\":\"acquisition\",\"name\":\"demo\",\"version\":\"0.1.0\",\"schema_version\":\"1\",\"capabilities\":[]}' '{\"output\":\"four\"}'",
                ),
            ],
            "input",
            "output",
            Duration::from_secs(1),
        );

        let output = acquisition.acquire(&String::from("2+2")).await.unwrap();

        assert_eq!(output, "four");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn subprocess_acquisition_maps_structured_plugin_errors() {
        let acquisition = SubprocessAcquisition::new(
            "sh",
            vec![
                String::from("-c"),
                String::from(
                    "read line; printf '%s\n' '{\"kind\":\"acquisition\",\"name\":\"demo\",\"version\":\"0.1.0\",\"schema_version\":\"1\",\"capabilities\":[\"structured-errors\"]}' '{\"error\":{\"code\":\"bad_input\",\"message\":\"oops\",\"details\":{\"field\":\"input\"}}}'",
                ),
            ],
            "input",
            "output",
            Duration::from_secs(1),
        );

        let err = acquisition.acquire(&String::from("bad")).await.unwrap_err();

        assert!(err.to_string().contains("plugin error [bad_input]: oops"));
    }
}
