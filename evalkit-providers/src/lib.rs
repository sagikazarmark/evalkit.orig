use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use evalkit::{Acquisition, AcquisitionError};
use reqwest::Client;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;

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

        let _ = child.wait().await;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(AcquisitionError::ExecutionFailed(Box::new(
                EmptyProcessOutput,
            )));
        }

        let payload: Value = serde_json::from_str(trimmed)
            .map_err(|source| AcquisitionError::ExecutionFailed(Box::new(source)))?;

        extract_string_field(&payload, &self.output_field)
    }
}

impl Acquisition<String, String> for SubprocessAcquisition {
    async fn acquire(&self, input: &String) -> Result<String, AcquisitionError> {
        tokio::time::timeout(self.timeout, self.run(input))
            .await
            .map_err(|_| AcquisitionError::Timeout(self.timeout))?
    }
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
}
