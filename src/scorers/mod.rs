use crate::{Score, ScoreDefinition, Scorer, ScorerContext, ScorerError};
use regex::Regex;
#[cfg(feature = "llm-judge")]
use reqwest::Client;
#[cfg(feature = "llm-judge")]
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub fn exact_match() -> impl Scorer<String, String, String> {
    ExactMatchScorer
}

pub fn contains() -> impl Scorer<String, String, String> {
    ContainsScorer
}

pub fn regex(pattern: &str) -> Result<impl Scorer<String, String>, regex::Error> {
    Ok(RegexScorer {
        pattern: Regex::new(pattern)?,
    })
}

pub fn json_schema(schema: Value) -> impl Scorer<String, String> {
    JsonSchemaScorer { schema }
}

#[cfg(feature = "llm-judge")]
pub fn llm_judge(config: LlmJudgeConfig) -> impl Scorer<String, String, String> {
    LlmJudgeScorer {
        client: Client::new(),
        config,
    }
}

#[cfg(feature = "llm-judge")]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmJudgeConfig {
    pub model: String,
    pub base_url: String,
    pub prompt_template: String,
    pub score_extractor: LlmJudgeScoreExtractor,
    #[serde(skip, default)]
    pub api_key: String,
}

#[cfg(feature = "llm-judge")]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmJudgeScoreExtractor {
    JsonScore,
    Boolean,
    Numeric,
    Label,
}

struct ExactMatchScorer;

impl Scorer<String, String, String> for ExactMatchScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        let reference = ctx.reference.ok_or_else(|| {
            ScorerError(Box::new(BuiltinScorerError::MissingReference {
                scorer_name: "exact_match",
            }))
        })?;

        Ok(Score::Binary(ctx.output == reference))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::new("exact_match")
    }
}

struct ContainsScorer;

impl Scorer<String, String, String> for ContainsScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        let reference = ctx.reference.ok_or_else(|| {
            ScorerError(Box::new(BuiltinScorerError::MissingReference {
                scorer_name: "contains",
            }))
        })?;

        Ok(Score::Binary(ctx.output.contains(reference)))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::new("contains")
    }
}

struct RegexScorer {
    pattern: Regex,
}

impl Scorer<String, String> for RegexScorer {
    async fn score(&self, ctx: &ScorerContext<'_, String, String>) -> Result<Score, ScorerError> {
        Ok(Score::Binary(self.pattern.is_match(ctx.output)))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::new("regex")
    }
}

struct JsonSchemaScorer {
    schema: Value,
}

#[cfg(feature = "llm-judge")]
struct LlmJudgeScorer {
    client: Client,
    config: LlmJudgeConfig,
}

impl Scorer<String, String> for JsonSchemaScorer {
    async fn score(&self, ctx: &ScorerContext<'_, String, String>) -> Result<Score, ScorerError> {
        let output = serde_json::from_str::<Value>(ctx.output).map_err(|source| {
            ScorerError(Box::new(BuiltinScorerError::InvalidJson {
                scorer_name: "json_schema",
                source,
            }))
        })?;

        let is_valid = validate_json_schema(&self.schema, &output).map_err(|message| {
            ScorerError(Box::new(BuiltinScorerError::InvalidSchema {
                scorer_name: "json_schema",
                message,
            }))
        })?;

        Ok(Score::Binary(is_valid))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::new("json_schema")
    }
}

#[cfg(feature = "llm-judge")]
impl Scorer<String, String, String> for LlmJudgeScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        let prompt = render_prompt(&self.config.prompt_template, ctx);
        let request = LlmJudgeRequest {
            model: self.config.model.clone(),
            messages: vec![LlmJudgeMessage {
                role: "user",
                content: prompt,
            }],
        };

        let response = self
            .client
            .post(chat_completions_url(&self.config.base_url))
            .bearer_auth(&self.config.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|source| {
                ScorerError(Box::new(LlmJudgeError::Network {
                    source,
                    model: self.config.model.clone(),
                }))
            })?
            .error_for_status()
            .map_err(|source| {
                ScorerError(Box::new(LlmJudgeError::Network {
                    source,
                    model: self.config.model.clone(),
                }))
            })?;

        let payload = response
            .json::<LlmJudgeResponse>()
            .await
            .map_err(|source| {
                ScorerError(Box::new(LlmJudgeError::Network {
                    source,
                    model: self.config.model.clone(),
                }))
            })?;

        let content = payload.first_message_content().map_err(|message| {
            ScorerError(Box::new(LlmJudgeError::InvalidResponse {
                model: self.config.model.clone(),
                message,
            }))
        })?;

        self.config
            .score_extractor
            .extract(content)
            .map_err(|message| {
                ScorerError(Box::new(LlmJudgeError::Parse {
                    model: self.config.model.clone(),
                    message,
                }))
            })
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::new("llm_judge")
    }
}

#[derive(Debug)]
enum BuiltinScorerError {
    MissingReference {
        scorer_name: &'static str,
    },
    InvalidJson {
        scorer_name: &'static str,
        source: serde_json::Error,
    },
    InvalidSchema {
        scorer_name: &'static str,
        message: String,
    },
}

#[cfg(feature = "llm-judge")]
#[derive(Debug)]
enum LlmJudgeError {
    Network {
        source: reqwest::Error,
        model: String,
    },
    InvalidResponse {
        model: String,
        message: String,
    },
    Parse {
        model: String,
        message: String,
    },
}

impl Display for BuiltinScorerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReference { scorer_name } => {
                write!(f, "{scorer_name} scorer requires a reference value")
            }
            Self::InvalidJson {
                scorer_name,
                source,
            } => write!(
                f,
                "{scorer_name} scorer received invalid JSON output: {source}"
            ),
            Self::InvalidSchema {
                scorer_name,
                message,
            } => write!(
                f,
                "{scorer_name} scorer received an invalid schema: {message}"
            ),
        }
    }
}

impl Error for BuiltinScorerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidJson { source, .. } => Some(source),
            Self::MissingReference { .. } | Self::InvalidSchema { .. } => None,
        }
    }
}

#[cfg(feature = "llm-judge")]
impl Display for LlmJudgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network { source, model } => {
                write!(
                    f,
                    "llm_judge scorer network failure for model `{model}`: {source}"
                )
            }
            Self::InvalidResponse { model, message } => write!(
                f,
                "llm_judge scorer received an invalid response from model `{model}`: {message}"
            ),
            Self::Parse { model, message } => {
                write!(
                    f,
                    "llm_judge scorer could not parse model `{model}` output: {message}"
                )
            }
        }
    }
}

#[cfg(feature = "llm-judge")]
impl Error for LlmJudgeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Network { source, .. } => Some(source),
            Self::InvalidResponse { .. } | Self::Parse { .. } => None,
        }
    }
}

#[cfg(feature = "llm-judge")]
#[derive(Serialize)]
struct LlmJudgeRequest {
    model: String,
    messages: Vec<LlmJudgeMessage<'static>>,
}

#[cfg(feature = "llm-judge")]
#[derive(Serialize)]
struct LlmJudgeMessage<'a> {
    role: &'a str,
    content: String,
}

#[cfg(feature = "llm-judge")]
#[derive(Deserialize)]
struct LlmJudgeResponse {
    choices: Vec<LlmJudgeChoice>,
}

#[cfg(feature = "llm-judge")]
#[derive(Deserialize)]
struct LlmJudgeChoice {
    message: LlmJudgeResponseMessage,
}

#[cfg(feature = "llm-judge")]
#[derive(Deserialize)]
struct LlmJudgeResponseMessage {
    content: String,
}

#[cfg(feature = "llm-judge")]
impl LlmJudgeResponse {
    fn first_message_content(&self) -> Result<&str, String> {
        self.choices
            .first()
            .map(|choice| choice.message.content.trim())
            .filter(|content| !content.is_empty())
            .ok_or_else(|| String::from("response did not include a non-empty assistant message"))
    }
}

#[cfg(feature = "llm-judge")]
impl LlmJudgeScoreExtractor {
    fn extract(&self, content: &str) -> Result<Score, String> {
        match self {
            Self::JsonScore => serde_json::from_str::<Score>(content)
                .map_err(|source| format!("expected Score JSON: {source}")),
            Self::Boolean => content
                .parse::<bool>()
                .map(Score::Binary)
                .map_err(|source| format!("expected `true` or `false`: {source}")),
            Self::Numeric => content
                .parse::<f64>()
                .map(Score::Numeric)
                .map_err(|source| format!("expected a numeric score: {source}")),
            Self::Label => {
                let label = content.trim();

                if label.is_empty() {
                    return Err(String::from("expected a non-empty label"));
                }

                Ok(Score::Label(label.to_string()))
            }
        }
    }
}

#[cfg(feature = "llm-judge")]
fn chat_completions_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

#[cfg(feature = "llm-judge")]
fn render_prompt(template: &str, ctx: &ScorerContext<'_, String, String, String>) -> String {
    template
        .replace("{{input}}", ctx.input)
        .replace("{{output}}", ctx.output)
        .replace(
            "{{reference}}",
            ctx.reference.map(String::as_str).unwrap_or(""),
        )
}

fn validate_json_schema(schema: &Value, instance: &Value) -> Result<bool, String> {
    match schema {
        Value::Bool(allowed) => Ok(*allowed),
        Value::Object(schema) => validate_schema_object(schema, instance),
        _ => Err(format!(
            "schema must be an object or boolean, got {}",
            json_type_name(schema)
        )),
    }
}

fn validate_schema_object(schema: &Map<String, Value>, instance: &Value) -> Result<bool, String> {
    if let Some(expected) = schema.get("const")
        && instance != expected
    {
        return Ok(false);
    }

    if let Some(variants) = schema.get("enum") {
        let variants = variants
            .as_array()
            .ok_or_else(|| schema_keyword_type_error("enum", "array", variants))?;

        if !variants.iter().any(|candidate| candidate == instance) {
            return Ok(false);
        }
    }

    if let Some(expected_type) = schema.get("type")
        && !matches_schema_type(expected_type, instance)?
    {
        return Ok(false);
    }

    if let Some(minimum) = schema.get("minimum")
        && let Some(value) = instance.as_f64()
    {
        let minimum = minimum
            .as_f64()
            .ok_or_else(|| schema_keyword_type_error("minimum", "number", minimum))?;

        if value < minimum {
            return Ok(false);
        }
    }

    if let Some(maximum) = schema.get("maximum")
        && let Some(value) = instance.as_f64()
    {
        let maximum = maximum
            .as_f64()
            .ok_or_else(|| schema_keyword_type_error("maximum", "number", maximum))?;

        if value > maximum {
            return Ok(false);
        }
    }

    if let Some(min_length) = schema.get("minLength")
        && let Some(value) = instance.as_str()
    {
        let min_length = min_length.as_u64().ok_or_else(|| {
            schema_keyword_type_error("minLength", "unsigned integer", min_length)
        })?;

        if value.chars().count() < min_length as usize {
            return Ok(false);
        }
    }

    if let Some(max_length) = schema.get("maxLength")
        && let Some(value) = instance.as_str()
    {
        let max_length = max_length.as_u64().ok_or_else(|| {
            schema_keyword_type_error("maxLength", "unsigned integer", max_length)
        })?;

        if value.chars().count() > max_length as usize {
            return Ok(false);
        }
    }

    if let Some(required) = schema.get("required") {
        let required = required
            .as_array()
            .ok_or_else(|| schema_keyword_type_error("required", "array", required))?;

        if let Some(object) = instance.as_object() {
            for property in required {
                let property = property
                    .as_str()
                    .ok_or_else(|| schema_keyword_type_error("required[]", "string", property))?;

                if !object.contains_key(property) {
                    return Ok(false);
                }
            }
        }
    }

    if let Some(properties) = schema.get("properties") {
        let properties = properties
            .as_object()
            .ok_or_else(|| schema_keyword_type_error("properties", "object", properties))?;

        if let Some(object) = instance.as_object() {
            for (property, property_schema) in properties {
                if let Some(value) = object.get(property)
                    && !validate_json_schema(property_schema, value)?
                {
                    return Ok(false);
                }
            }
        }
    }

    if let Some(items) = schema.get("items")
        && let Some(values) = instance.as_array()
    {
        for value in values {
            if !validate_json_schema(items, value)? {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn matches_schema_type(expected_type: &Value, instance: &Value) -> Result<bool, String> {
    match expected_type {
        Value::String(expected_type) => matches_schema_type_name(expected_type, instance),
        Value::Array(expected_types) => {
            for expected_type in expected_types {
                let expected_type = expected_type
                    .as_str()
                    .ok_or_else(|| schema_keyword_type_error("type[]", "string", expected_type))?;

                if matches_schema_type_name(expected_type, instance)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        _ => Err(schema_keyword_type_error(
            "type",
            "string or array of strings",
            expected_type,
        )),
    }
}

fn matches_schema_type_name(expected_type: &str, instance: &Value) -> Result<bool, String> {
    match expected_type {
        "null" => Ok(instance.is_null()),
        "boolean" => Ok(instance.is_boolean()),
        "object" => Ok(instance.is_object()),
        "array" => Ok(instance.is_array()),
        "number" => Ok(instance.is_number()),
        "integer" => Ok(is_json_integer(instance)),
        "string" => Ok(instance.is_string()),
        _ => Err(format!("unsupported schema type `{expected_type}`")),
    }
}

fn is_json_integer(value: &Value) -> bool {
    match value {
        Value::Number(number) => {
            number.as_i64().is_some()
                || number.as_u64().is_some()
                || number.as_f64().is_some_and(|value| value.fract() == 0.0)
        }
        _ => false,
    }
}

fn schema_keyword_type_error(keyword: &str, expected: &str, actual: &Value) -> String {
    format!(
        "`{keyword}` must be {expected}, got {}",
        json_type_name(actual)
    )
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::{contains, exact_match, json_schema, regex};
    use crate::{Score, ScoreDefinition, Scorer, ScorerContext};
    use serde_json::json;

    fn string_context<'a>(
        input: &'a String,
        output: &'a String,
        reference: Option<&'a String>,
    ) -> ScorerContext<'a, String, String, String> {
        ScorerContext {
            input,
            output,
            reference,
        }
    }

    fn string_context_without_reference<'a>(
        input: &'a String,
        output: &'a String,
    ) -> ScorerContext<'a, String, String> {
        ScorerContext {
            input,
            output,
            reference: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exact_match_returns_binary_score() {
        let input = String::from("What is 2 + 2?");
        let output = String::from("4");
        let reference = String::from("4");
        let scorer = exact_match();

        let score = scorer
            .score(&string_context(&input, &output, Some(&reference)))
            .await
            .unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn contains_returns_binary_score() {
        let input = String::from("question");
        let output = String::from("The capital is Paris, France.");
        let reference = String::from("Paris");
        let scorer = contains();

        let score = scorer
            .score(&string_context(&input, &output, Some(&reference)))
            .await
            .unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn regex_returns_binary_score() {
        let input = String::from("prompt");
        let output = String::from("Order #314 is ready");
        let scorer = regex(r"#\d+").unwrap();

        let score = scorer
            .score(&string_context_without_reference(&input, &output))
            .await
            .unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn missing_reference_returns_scorer_error() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let scorer = exact_match();
        let ctx = ScorerContext {
            input: &input,
            output: &output,
            reference: None,
        };

        let err = scorer.score(&ctx).await.unwrap_err();

        assert_eq!(
            err.to_string(),
            "exact_match scorer requires a reference value"
        );
    }

    #[test]
    fn invalid_regex_pattern_is_distinct_from_low_score() {
        let invalid_pattern = regex("(").err().expect("pattern should be invalid");
        let non_match = regex("^done$");

        assert!(invalid_pattern.to_string().contains("unclosed group"));
        assert!(non_match.is_ok());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn json_schema_validates_object_shape() {
        let input = String::from("prompt");
        let output = String::from(r#"{"answer":"Paris","confidence":0.9}"#);
        let scorer = json_schema(json!({
            "type": "object",
            "required": ["answer", "confidence"],
            "properties": {
                "answer": { "type": "string", "minLength": 1 },
                "confidence": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
            }
        }));

        let score = scorer
            .score(&string_context_without_reference(&input, &output))
            .await
            .unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn json_schema_returns_false_for_non_matching_json() {
        let input = String::from("prompt");
        let output = String::from(r#"{"answer":"","confidence":1.5}"#);
        let scorer = json_schema(json!({
            "type": "object",
            "required": ["answer", "confidence"],
            "properties": {
                "answer": { "type": "string", "minLength": 1 },
                "confidence": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
            }
        }));

        let score = scorer
            .score(&string_context_without_reference(&input, &output))
            .await
            .unwrap();

        assert_eq!(score, Score::Binary(false));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn json_schema_invalid_json_is_a_scorer_error() {
        let input = String::from("prompt");
        let output = String::from("not-json");
        let scorer = json_schema(json!({ "type": "object" }));

        let err = scorer
            .score(&string_context_without_reference(&input, &output))
            .await
            .unwrap_err();

        assert!(
            err.to_string()
                .starts_with("json_schema scorer received invalid JSON output:")
        );
    }

    #[test]
    fn builtin_scorers_expose_expected_definitions() {
        assert_eq!(
            exact_match().definition(),
            ScoreDefinition::new("exact_match")
        );
        assert_eq!(contains().definition(), ScoreDefinition::new("contains"));
        assert_eq!(
            regex(".*").unwrap().definition(),
            ScoreDefinition::new("regex")
        );
        assert_eq!(
            json_schema(json!({})).definition(),
            ScoreDefinition::new("json_schema")
        );
    }
}
