use anyllm::{
    ChatProvider, ChatRequest, DynChatProvider, ExtractExt, ProviderIdentity, ReasoningConfig,
};
use evalkit::{
    Score, ScoreDefinition, ScoreOutcome, Scorer, ScorerContext, ScorerError, ScorerMetadata,
    ScorerResources, TokenUsage,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

const LLM_CLASSIFIER_PROMPT_TEMPLATE: &str = include_str!("../prompts/llm_classifier.txt");
const G_EVAL_PROMPT_TEMPLATE: &str = include_str!("../prompts/g_eval.txt");

/// Creates a generic LLM-as-a-Judge scorer backed by `anyllm`.
pub fn llm_judge<P>(
    provider: P,
    model: impl Into<String>,
    prompt: impl Into<PromptTemplate>,
    output: LlmJudgeOutput,
) -> LlmJudge
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
{
    LlmJudge::new(provider, model, prompt, output)
}

/// Creates a closed-set label classifier backed by `LlmJudge`.
pub fn llm_classifier<P, I, S>(
    provider: P,
    model: impl Into<String>,
    labels: I,
    instructions: impl Into<String>,
) -> Result<LlmJudge, LlmJudgeBuildError>
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let labels = normalize_classifier_labels(labels.into_iter().map(ClassifierLabel::new))?;

    Ok(llm_classifier_with_normalized_labels(
        provider,
        model,
        labels,
        instructions.into(),
    ))
}

/// Creates a closed-set classifier with richer per-label descriptions.
pub fn llm_classifier_with_labels<P, I>(
    provider: P,
    model: impl Into<String>,
    labels: I,
    instructions: impl Into<String>,
) -> Result<LlmJudge, LlmJudgeBuildError>
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
    I: IntoIterator<Item = ClassifierLabel>,
{
    let labels = normalize_classifier_labels(labels)?;

    Ok(llm_classifier_with_normalized_labels(
        provider,
        model,
        labels,
        instructions.into(),
    ))
}

fn llm_classifier_with_normalized_labels<P>(
    provider: P,
    model: impl Into<String>,
    labels: Vec<ClassifierLabel>,
    instructions: String,
) -> LlmJudge
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
{
    let prompt = build_llm_classifier_prompt(&labels, instructions);

    LlmJudge::new(provider, model, prompt, LlmJudgeOutput::Label).named("llm_classifier")
}

/// Creates a first-pass G-Eval scorer backed by `LlmJudge`.
pub fn g_eval<P, I, S>(
    provider: P,
    model: impl Into<String>,
    criteria: I,
) -> Result<LlmJudge, LlmJudgeBuildError>
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let criteria = normalize_entries("criteria", criteria)?;
    let steps = build_g_eval_steps(&criteria);

    Ok(g_eval_with_normalized_steps(
        provider, model, criteria, steps,
    ))
}

/// Creates a G-Eval scorer with caller-specified evaluation steps.
pub fn g_eval_with_steps<P, I, S, J, T>(
    provider: P,
    model: impl Into<String>,
    criteria: I,
    steps: J,
) -> Result<LlmJudge, LlmJudgeBuildError>
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
    I: IntoIterator<Item = S>,
    S: Into<String>,
    J: IntoIterator<Item = T>,
    T: Into<String>,
{
    let criteria = normalize_entries("criteria", criteria)?;
    let steps = normalize_entries("steps", steps)?;

    Ok(g_eval_with_normalized_steps(
        provider, model, criteria, steps,
    ))
}

fn g_eval_with_normalized_steps<P>(
    provider: P,
    model: impl Into<String>,
    criteria: Vec<String>,
    steps: Vec<String>,
) -> LlmJudge
where
    P: ChatProvider + 'static,
    P::Stream: 'static,
{
    let prompt = build_g_eval_prompt(&criteria, &steps);

    LlmJudge::new(provider, model, prompt, LlmJudgeOutput::Numeric)
        .named("g_eval")
        .capture_reasoning(true)
}

/// Stable prompt template with a canonical form for hashing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTemplate {
    raw: String,
    canonical: String,
}

impl PromptTemplate {
    /// Creates a prompt template and computes its canonical form.
    pub fn new(template: impl Into<String>) -> Self {
        let raw = template.into();
        let canonical = canonicalize_prompt(&raw);

        Self { raw, canonical }
    }

    /// Returns the original prompt text.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Returns the canonical prompt text used for hashing.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Returns a stable hash of the canonical prompt text.
    pub fn prompt_hash(&self) -> String {
        let digest = Sha256::digest(self.canonical.as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);

        for byte in digest {
            use std::fmt::Write as _;

            let _ = write!(hex, "{byte:02x}");
        }

        hex
    }

    fn render<I, O, R>(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<String, PromptRenderError>
    where
        I: Serialize,
        O: Serialize,
        R: Serialize,
    {
        let mut rendered = self.raw.clone();
        let replacements = [
            ("{{input}}", render_value("input", ctx.input)?),
            ("{{output}}", render_value("output", ctx.output)?),
            (
                "{{reference}}",
                match ctx.reference {
                    Some(reference) => render_value("reference", reference)?,
                    None => String::new(),
                },
            ),
            ("{{run_id}}", ctx.run_id.to_owned()),
            ("{{sample_id}}", ctx.sample_id.to_owned()),
            ("{{trial_index}}", ctx.trial_index.to_string()),
            (
                "{{metadata}}",
                render_json_value("metadata", json!(ctx.metadata))?,
            ),
        ];

        for (placeholder, value) in replacements {
            rendered = rendered.replace(placeholder, &value);
        }

        Ok(rendered)
    }
}

impl From<String> for PromptTemplate {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for PromptTemplate {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Closed-set label definition for `llm_classifier`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ClassifierLabel {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ClassifierLabel {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Supported score shapes for `LlmJudge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmJudgeOutput {
    Binary,
    Numeric,
    Label,
}

impl LlmJudgeOutput {
    fn definition(self, name: &str) -> ScoreDefinition {
        match self {
            Self::Binary | Self::Numeric => ScoreDefinition::maximize(name),
            Self::Label => ScoreDefinition::new(name),
        }
    }
}

/// Provider-neutral LLM-as-a-Judge primitive using `anyllm` structured extraction.
#[derive(Debug)]
pub struct LlmJudge {
    provider: DynChatProvider,
    definition: ScoreDefinition,
    metadata: ScorerMetadata,
    model: String,
    prompt: PromptTemplate,
    output: LlmJudgeOutput,
    retries: usize,
    timeout: Option<Duration>,
    temperature: f32,
    max_tokens: Option<u32>,
    seed: Option<u64>,
    reasoning: Option<ReasoningConfig>,
    capture_reasoning: bool,
}

impl LlmJudge {
    /// Creates a new judge using the provided `anyllm` chat provider.
    pub fn new<P>(
        provider: P,
        model: impl Into<String>,
        prompt: impl Into<PromptTemplate>,
        output: LlmJudgeOutput,
    ) -> Self
    where
        P: ChatProvider + 'static,
        P::Stream: 'static,
    {
        let model = model.into();
        let provider = DynChatProvider::new(provider);
        let metadata = ScorerMetadata::default().judge_model_pin(format!(
            "{}/{}",
            provider.provider_name(),
            model
        ));

        Self {
            provider,
            definition: output.definition("llm_judge"),
            metadata,
            model,
            prompt: prompt.into(),
            output,
            retries: 0,
            timeout: None,
            temperature: 0.0,
            max_tokens: None,
            seed: None,
            reasoning: None,
            capture_reasoning: false,
        }
    }

    /// Overrides the exported scorer name.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.definition = self.output.definition(&name.into());
        self
    }

    /// Overrides the scorer prompt template.
    pub fn with_prompt(mut self, prompt: impl Into<PromptTemplate>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Sets how many additional attempts to make after a failed provider call.
    pub fn retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    /// Sets a timeout for each provider attempt.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the judge sampling temperature. Defaults to `0.0` for determinism.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the max output tokens for the judge request.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the deterministic seed for providers that support it.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Configures provider-agnostic reasoning options for the judge request.
    pub fn reasoning(mut self, reasoning: impl Into<ReasoningConfig>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    /// Returns `Score::Structured` with reasoning for numeric and binary outputs.
    pub fn capture_reasoning(mut self, capture_reasoning: bool) -> Self {
        self.capture_reasoning = capture_reasoning;
        self
    }

    /// Returns the stable prompt hash for this scorer instance.
    pub fn prompt_hash(&self) -> String {
        self.prompt.prompt_hash()
    }

    fn request<I, O, R>(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<ChatRequest, ScorerError>
    where
        I: Serialize,
        O: Serialize,
        R: Serialize,
    {
        let rendered = self.prompt.render(ctx).map_err(ScorerError::internal)?;

        let mut request = ChatRequest::new(self.model.clone())
            .temperature(self.temperature)
            .user(rendered);

        if let Some(max_tokens) = self.max_tokens {
            request = request.max_tokens(max_tokens);
        }

        if let Some(seed) = self.seed {
            request = request.seed(seed);
        }

        if let Some(reasoning) = &self.reasoning {
            request = request.reasoning(reasoning.clone());
        }

        Ok(request)
    }

    async fn extract_with_retry<T>(
        &self,
        request: &ChatRequest,
    ) -> Result<anyllm::Extracted<T>, ScorerError>
    where
        T: JsonSchema + for<'de> Deserialize<'de> + Send,
    {
        let attempts = self.retries + 1;
        let mut last_error = None;

        for _ in 0..attempts {
            let result = if let Some(timeout) = self.timeout {
                match tokio::time::timeout(timeout, self.provider.extract::<T>(request)).await {
                    Ok(result) => result,
                    Err(_) => return Err(ScorerError::Timeout(timeout)),
                }
            } else {
                self.provider.extract::<T>(request).await
            };

            match result {
                Ok(extracted) => return Ok(extracted),
                Err(error) => last_error = Some(error),
            }
        }

        Err(ScorerError::provider(
            last_error.expect("at least one attempt is executed"),
        ))
    }
}

impl<I, O, R> Scorer<I, O, R> for LlmJudge
where
    I: Serialize,
    O: Serialize,
    R: Serialize,
{
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError> {
        Ok(self.score_with_resources(ctx).await?.score)
    }

    async fn score_with_resources(
        &self,
        ctx: &ScorerContext<'_, I, O, R>,
    ) -> Result<ScoreOutcome, ScorerError> {
        if self.capture_reasoning && self.output == LlmJudgeOutput::Label {
            return Err(ScorerError::invalid_input(
                LlmJudgeConfigurationError::LabelReasoningCaptureUnsupported,
            ));
        }

        let request = self.request(ctx)?;

        match self.output {
            LlmJudgeOutput::Binary => {
                let extracted = self
                    .extract_with_retry::<BinaryJudgeResponse>(&request)
                    .await?;
                Ok(binary_score(
                    extracted.value,
                    &extracted.response,
                    &self.prompt,
                    &self.provider,
                    self.capture_reasoning,
                ))
            }
            LlmJudgeOutput::Numeric => {
                let extracted = self
                    .extract_with_retry::<NumericJudgeResponse>(&request)
                    .await?;
                Ok(numeric_score(
                    extracted.value,
                    &extracted.response,
                    &self.prompt,
                    &self.provider,
                    self.capture_reasoning,
                ))
            }
            LlmJudgeOutput::Label => {
                let extracted = self
                    .extract_with_retry::<LabelJudgeResponse>(&request)
                    .await?;
                Ok(ScoreOutcome::new(Score::Label(extracted.value.label))
                    .with_resources(response_resources(&extracted.response)))
            }
        }
    }

    fn definition(&self) -> ScoreDefinition {
        self.definition.clone()
    }

    fn metadata(&self) -> ScorerMetadata {
        self.metadata.clone()
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct BinaryJudgeResponse {
    score: bool,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NumericJudgeResponse {
    score: f64,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LabelJudgeResponse {
    label: String,
}

fn binary_score(
    result: BinaryJudgeResponse,
    response: &anyllm::ChatResponse,
    prompt: &PromptTemplate,
    provider: &DynChatProvider,
    capture_reasoning: bool,
) -> ScoreOutcome {
    let resources = response_resources(response);

    if capture_reasoning {
        ScoreOutcome::new(Score::Structured {
            score: if result.score { 1.0 } else { 0.0 },
            reasoning: result.reasoning.unwrap_or_default(),
            metadata: judge_metadata(
                prompt,
                provider,
                response,
                "binary",
                json!({ "binary": result.score }),
                result.metadata,
            ),
        })
        .with_resources(resources)
    } else {
        ScoreOutcome::new(Score::Binary(result.score)).with_resources(resources)
    }
}

fn numeric_score(
    result: NumericJudgeResponse,
    response: &anyllm::ChatResponse,
    prompt: &PromptTemplate,
    provider: &DynChatProvider,
    capture_reasoning: bool,
) -> ScoreOutcome {
    let resources = response_resources(response);

    if capture_reasoning {
        ScoreOutcome::new(Score::Structured {
            score: result.score,
            reasoning: result.reasoning.unwrap_or_default(),
            metadata: judge_metadata(
                prompt,
                provider,
                response,
                "numeric",
                json!({ "score": result.score }),
                result.metadata,
            ),
        })
        .with_resources(resources)
    } else {
        ScoreOutcome::new(Score::Numeric(result.score)).with_resources(resources)
    }
}

fn response_resources(response: &anyllm::ChatResponse) -> ScorerResources {
    let Some(usage) = response.usage.as_ref() else {
        return ScorerResources::default();
    };

    ScorerResources::default().token_usage(TokenUsage {
        input: usage.input_tokens.unwrap_or(0),
        output: usage.output_tokens.unwrap_or(0),
        cache_read: usage.cached_input_tokens.unwrap_or(0),
        cache_write: usage.cache_creation_input_tokens.unwrap_or(0),
    })
}

fn judge_metadata(
    prompt: &PromptTemplate,
    provider: &DynChatProvider,
    response: &anyllm::ChatResponse,
    output_kind: &str,
    extracted: Value,
    result_metadata: Option<Value>,
) -> Value {
    json!({
        "judge": {
            "provider": provider.provider_name(),
            "model": response.model.clone(),
            "response_id": response.id.clone(),
            "output_kind": output_kind,
            "prompt_hash": prompt.prompt_hash(),
            "usage": response.usage.as_ref().map(|usage| json!({
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "total_tokens": usage.total_tokens,
                "cached_input_tokens": usage.cached_input_tokens,
                "cache_creation_input_tokens": usage.cache_creation_input_tokens,
                "reasoning_tokens": usage.reasoning_tokens,
            })),
            "extracted": extracted,
            "result_metadata": result_metadata,
        }
    })
}

fn render_value<T>(field: &'static str, value: &T) -> Result<String, PromptRenderError>
where
    T: Serialize,
{
    let value =
        serde_json::to_value(value).map_err(|source| PromptRenderError { field, source })?;
    render_json_value(field, value)
}

fn render_json_value(field: &'static str, value: Value) -> Result<String, PromptRenderError> {
    match value {
        Value::String(text) => Ok(text),
        other => serde_json::to_string_pretty(&other)
            .map_err(|source| PromptRenderError { field, source }),
    }
}

fn canonicalize_prompt(template: &str) -> String {
    let normalized = template.replace("\r\n", "\n").replace('\r', "\n");
    let mut canonical = String::new();
    let mut pending_blank = false;

    for line in normalized.lines() {
        let trimmed_end = line.trim_end();

        if trimmed_end.trim().is_empty() {
            pending_blank = !canonical.is_empty();
            continue;
        }

        if pending_blank {
            canonical.push_str("\n\n");
            pending_blank = false;
        } else if !canonical.is_empty() {
            canonical.push('\n');
        }

        canonical.push_str(trimmed_end);
    }

    canonical.trim().to_owned()
}

#[derive(Debug)]
struct PromptRenderError {
    field: &'static str,
    source: serde_json::Error,
}

impl Display for PromptRenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to render prompt field '{}': {}",
            self.field, self.source
        )
    }
}

impl Error for PromptRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
enum LlmJudgeConfigurationError {
    LabelReasoningCaptureUnsupported,
}

impl Display for LlmJudgeConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LabelReasoningCaptureUnsupported => f.write_str(
                "label judges cannot capture reasoning because Score::Structured requires a numeric score",
            ),
        }
    }
}

impl Error for LlmJudgeConfigurationError {}

/// Construction errors for higher-level LLM judge wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmJudgeBuildError {
    EmptyEntries { field: &'static str },
    BlankEntry { field: &'static str, index: usize },
    DuplicateEntry { field: &'static str, value: String },
}

impl Display for LlmJudgeBuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEntries { field } => write!(f, "{field} must contain at least one entry"),
            Self::BlankEntry { field, index } => {
                write!(f, "{field}[{index}] must not be blank")
            }
            Self::DuplicateEntry { field, value } => {
                write!(f, "{field} must not contain duplicate entry '{value}'")
            }
        }
    }
}

impl Error for LlmJudgeBuildError {}

fn build_llm_classifier_prompt(labels: &[ClassifierLabel], instructions: String) -> PromptTemplate {
    let labels_json =
        serde_json::to_string(labels).expect("labels serialize into JSON array of objects");
    let prompt = LLM_CLASSIFIER_PROMPT_TEMPLATE
        .replace("{{labels_bullets}}", &render_classifier_labels(labels))
        .replace("{{labels_json}}", &labels_json)
        .replace("{{instructions}}", instructions.trim());

    PromptTemplate::new(prompt)
}

fn build_g_eval_prompt(criteria: &[String], steps: &[String]) -> PromptTemplate {
    let criteria_json =
        serde_json::to_string(criteria).expect("criteria serialize into JSON array");
    let steps_json = serde_json::to_string(steps).expect("steps serialize into JSON array");
    let prompt = G_EVAL_PROMPT_TEMPLATE
        .replace("{{criteria_numbered}}", &render_numbered_list(criteria))
        .replace("{{criteria_json}}", &criteria_json)
        .replace("{{steps_numbered}}", &render_numbered_list(steps))
        .replace("{{steps_json}}", &steps_json);

    PromptTemplate::new(prompt)
}

fn build_g_eval_steps(criteria: &[String]) -> Vec<String> {
    let mut steps = Vec::with_capacity(criteria.len() + 2);
    steps.push(String::from(
        "Read the input, candidate output, and reference before scoring.",
    ));
    steps.extend(
        criteria.iter().map(|criterion| {
            format!("Assess whether the candidate output satisfies: {criterion}.")
        }),
    );
    steps.push(String::from(
        "Combine the step findings into a single score from 0.0 to 1.0.",
    ));
    steps
}

fn normalize_entries<I, S>(
    field: &'static str,
    entries: I,
) -> Result<Vec<String>, LlmJudgeBuildError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut normalized = Vec::new();

    for (index, entry) in entries.into_iter().enumerate() {
        let entry = entry.into();
        let trimmed = entry.trim();

        if trimmed.is_empty() {
            return Err(LlmJudgeBuildError::BlankEntry { field, index });
        }

        normalized.push(trimmed.to_owned());
    }

    if normalized.is_empty() {
        return Err(LlmJudgeBuildError::EmptyEntries { field });
    }

    Ok(normalized)
}

fn normalize_classifier_labels<I>(labels: I) -> Result<Vec<ClassifierLabel>, LlmJudgeBuildError>
where
    I: IntoIterator<Item = ClassifierLabel>,
{
    let mut normalized = Vec::new();

    for (index, label) in labels.into_iter().enumerate() {
        let name = label.label.trim();

        if name.is_empty() {
            return Err(LlmJudgeBuildError::BlankEntry {
                field: "labels",
                index,
            });
        }

        if normalized
            .iter()
            .any(|existing: &ClassifierLabel| existing.label == name)
        {
            return Err(LlmJudgeBuildError::DuplicateEntry {
                field: "labels",
                value: name.to_owned(),
            });
        }

        let description = match label.description {
            Some(description) => {
                let trimmed = description.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            }
            None => None,
        };

        normalized.push(ClassifierLabel {
            label: name.to_owned(),
            description,
        });
    }

    if normalized.is_empty() {
        return Err(LlmJudgeBuildError::EmptyEntries { field: "labels" });
    }

    Ok(normalized)
}

fn render_classifier_labels(labels: &[ClassifierLabel]) -> String {
    let mut rendered = String::new();

    for label in labels {
        rendered.push_str("- ");
        rendered.push_str(&label.label);

        if let Some(description) = &label.description {
            rendered.push_str(": ");
            rendered.push_str(description);
        }

        rendered.push('\n');
    }

    rendered.trim_end().to_owned()
}

fn render_numbered_list(entries: &[String]) -> String {
    let mut rendered = String::new();

    for (index, entry) in entries.iter().enumerate() {
        use std::fmt::Write as _;

        let _ = writeln!(rendered, "{}. {}", index + 1, entry);
    }

    rendered.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        ClassifierLabel, LlmJudgeBuildError, LlmJudgeOutput, PromptTemplate, g_eval,
        g_eval_with_steps, llm_classifier, llm_classifier_with_labels, llm_judge,
    };
    use anyllm::{
        ChatCapability, ChatResponseBuilder, Error as AnyLlmError, MockProvider, ResponseFormat,
        Usage, UserContent,
    };
    use evalkit::{Score, Scorer, ScorerContext};

    #[test]
    fn prompt_template_hash_uses_canonical_form() {
        let left = PromptTemplate::new("  Judge this.  \n\nOutput: {{output}}\n");
        let right = PromptTemplate::new("Judge this.\n\nOutput: {{output}}\r\n");

        assert_eq!(left.canonical(), right.canonical());
        assert_eq!(left.prompt_hash(), right.prompt_hash());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn llm_judge_extracts_numeric_score_with_structured_reasoning() {
        let provider = MockProvider::with_response(
            ChatResponseBuilder::new()
                .text(r#"{"score":0.75,"reasoning":"Grounded overall.","metadata":{"rubric":"faithfulness"}}"#)
                .usage_value(
                    Usage::new()
                        .input_tokens(12)
                        .output_tokens(4)
                        .cached_input_tokens(2),
                )
                .model("judge-model-response")
                .id("resp_1")
                .build(),
        )
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = llm_judge(
            provider.clone(),
            "judge-model",
            "Input: {{input}}\nOutput: {{output}}\nReference: {{reference}}",
            LlmJudgeOutput::Numeric,
        )
        .capture_reasoning(true)
        .max_tokens(128)
        .seed(7);

        let input = "question".to_string();
        let output = "answer".to_string();
        let reference = "gold".to_string();
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let outcome = scorer.score_with_resources(&ctx).await.unwrap();

        match outcome.score {
            Score::Structured {
                score,
                reasoning,
                metadata,
            } => {
                assert_eq!(score, 0.75);
                assert_eq!(reasoning, "Grounded overall.");
                assert_eq!(metadata["judge"]["provider"], "mock");
                assert_eq!(metadata["judge"]["output_kind"], "numeric");
                assert_eq!(metadata["judge"]["response_id"], "resp_1");
                assert_eq!(metadata["judge"]["usage"]["input_tokens"], 12);
                assert_eq!(
                    metadata["judge"]["result_metadata"]["rubric"],
                    "faithfulness"
                );
            }
            other => panic!("expected structured score, got {other:?}"),
        }

        assert_eq!(outcome.resources.token_usage.input, 12);
        assert_eq!(outcome.resources.token_usage.output, 4);
        assert_eq!(outcome.resources.token_usage.cache_read, 2);

        let request = provider.last_request().unwrap();
        assert_eq!(request.model, "judge-model");
        assert_eq!(request.temperature, Some(0.0));
        assert_eq!(request.max_tokens, Some(128));
        assert_eq!(request.seed, Some(7));
        assert!(matches!(
            request.response_format,
            Some(ResponseFormat::JsonSchema { .. })
        ));

        let user = request.messages[0].as_user().unwrap();
        let UserContent::Text(rendered) = user.content else {
            panic!("expected text user content")
        };
        assert!(rendered.contains("Input: question"));
        assert!(rendered.contains("Output: answer"));
        assert!(rendered.contains("Reference: gold"));
        assert_eq!(
            <super::LlmJudge as Scorer<String, String, String>>::metadata(&scorer).judge_model_pins,
            vec!["mock/judge-model".to_string()]
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn llm_judge_extracts_binary_score_without_reasoning_capture() {
        let provider = MockProvider::with_response(
            ChatResponseBuilder::new()
                .text(r#"{"score":true,"reasoning":"correct"}"#)
                .build(),
        )
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = llm_judge(
            provider,
            "judge-model",
            "Output: {{output}}",
            LlmJudgeOutput::Binary,
        );

        let input = String::new();
        let output = "match".to_string();
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let score = scorer.score(&ctx).await.unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn llm_judge_maps_provider_errors_into_scorer_errors() {
        let provider = MockProvider::new([
            AnyLlmError::Timeout("slow".into()),
            AnyLlmError::Timeout("slow".into()),
        ])
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = llm_judge(
            provider.clone(),
            "judge-model",
            "Output: {{output}}",
            LlmJudgeOutput::Numeric,
        )
        .retries(1);

        let input = String::new();
        let output = "answer".to_string();
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let err = scorer.score(&ctx).await.unwrap_err();

        assert!(matches!(err, evalkit::ScorerError::ProviderError(_)));
        assert_eq!(err.to_string(), "timeout: slow");
        assert_eq!(provider.call_count(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn llm_judge_rejects_label_reasoning_capture() {
        let provider = MockProvider::with_text(r#"{"label":"pass"}"#)
            .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = llm_judge(
            provider,
            "judge-model",
            "Output: {{output}}",
            LlmJudgeOutput::Label,
        )
        .capture_reasoning(true);

        let input = String::new();
        let output = "answer".to_string();
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let err = scorer.score(&ctx).await.unwrap_err();

        assert!(matches!(err, evalkit::ScorerError::InvalidInput(_)));
        assert!(
            err.to_string()
                .contains("label judges cannot capture reasoning")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn llm_classifier_renders_labels_and_returns_label_score() {
        let provider = MockProvider::with_response(
            ChatResponseBuilder::new()
                .text(r#"{"label":"approve"}"#)
                .build(),
        )
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = llm_classifier(
            provider.clone(),
            "judge-model",
            ["approve", "reject"],
            "Classify whether the answer should be accepted.",
        )
        .unwrap();

        let input = "question".to_string();
        let output = "answer".to_string();
        let reference = "gold".to_string();
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let score = scorer.score(&ctx).await.unwrap();

        assert_eq!(score, Score::Label("approve".to_string()));

        let request = provider.last_request().unwrap();
        let user = request.messages[0].as_user().unwrap();
        let UserContent::Text(rendered) = user.content else {
            panic!("expected text user content")
        };
        assert!(rendered.contains("- approve"));
        assert!(rendered.contains("- reject"));
        assert!(rendered.contains("[{\"label\":\"approve\"},{\"label\":\"reject\"}]"));
        assert!(rendered.contains("Classify whether the answer should be accepted."));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn llm_classifier_with_labels_renders_descriptions() {
        let provider = MockProvider::with_response(
            ChatResponseBuilder::new()
                .text(r#"{"label":"reject"}"#)
                .build(),
        )
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = llm_classifier_with_labels(
            provider.clone(),
            "judge-model",
            [
                ClassifierLabel::new("approve")
                    .description("Use when the answer is correct and grounded."),
                ClassifierLabel::new("reject")
                    .description("Use when the answer is wrong or unsupported."),
            ],
            "Classify whether the answer should be accepted.",
        )
        .unwrap();

        let input = "question".to_string();
        let output = "answer".to_string();
        let reference = "gold".to_string();
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let score = scorer.score(&ctx).await.unwrap();

        assert_eq!(score, Score::Label("reject".to_string()));

        let request = provider.last_request().unwrap();
        let user = request.messages[0].as_user().unwrap();
        let UserContent::Text(rendered) = user.content else {
            panic!("expected text user content")
        };
        assert!(rendered.contains("- approve: Use when the answer is correct and grounded."));
        assert!(rendered.contains("- reject: Use when the answer is wrong or unsupported."));
        assert!(
            rendered.contains("\"description\":\"Use when the answer is wrong or unsupported.\"")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn g_eval_renders_criteria_and_returns_structured_score() {
        let provider = MockProvider::with_response(
            ChatResponseBuilder::new()
                .text(r#"{"score":0.9,"reasoning":"The answer is correct and grounded.","metadata":{"criteria":["correctness","groundedness"],"method":"g_eval"}}"#)
                .build(),
        )
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = g_eval(
            provider.clone(),
            "judge-model",
            ["correctness", "groundedness"],
        )
        .unwrap();

        let input = "question".to_string();
        let output = "answer".to_string();
        let reference = "gold".to_string();
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let score = scorer.score(&ctx).await.unwrap();

        match score {
            Score::Structured {
                score,
                reasoning,
                metadata,
            } => {
                assert_eq!(score, 0.9);
                assert_eq!(reasoning, "The answer is correct and grounded.");
                assert_eq!(metadata["judge"]["result_metadata"]["method"], "g_eval");
                assert_eq!(
                    metadata["judge"]["result_metadata"]["criteria"],
                    serde_json::json!(["correctness", "groundedness"])
                );
            }
            other => panic!("expected structured score, got {other:?}"),
        }

        let request = provider.last_request().unwrap();
        let user = request.messages[0].as_user().unwrap();
        let UserContent::Text(rendered) = user.content else {
            panic!("expected text user content")
        };
        assert!(rendered.contains("1. correctness"));
        assert!(rendered.contains("2. groundedness"));
        assert!(
            rendered.contains("Read the input, candidate output, and reference before scoring.")
        );
        assert!(rendered.contains("Assess whether the candidate output satisfies: correctness."));
        assert!(
            rendered.contains("Combine the step findings into a single score from 0.0 to 1.0.")
        );
        assert!(rendered.contains("[\"correctness\",\"groundedness\"]"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn g_eval_with_steps_uses_custom_steps() {
        let provider = MockProvider::with_response(
            ChatResponseBuilder::new()
                .text(r#"{"score":0.7,"reasoning":"custom flow","metadata":{"method":"g_eval"}}"#)
                .build(),
        )
        .with_supported_chat_capabilities([ChatCapability::StructuredOutput]);

        let scorer = g_eval_with_steps(
            provider.clone(),
            "judge-model",
            ["correctness"],
            [
                "Check factual correctness against the reference.",
                "Penalize unsupported claims.",
            ],
        )
        .unwrap();

        let input = "question".to_string();
        let output = "answer".to_string();
        let reference = "gold".to_string();
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let _ = scorer.score(&ctx).await.unwrap();

        let request = provider.last_request().unwrap();
        let user = request.messages[0].as_user().unwrap();
        let UserContent::Text(rendered) = user.content else {
            panic!("expected text user content")
        };
        assert!(rendered.contains("Check factual correctness against the reference."));
        assert!(rendered.contains("Penalize unsupported claims."));
        assert!(
            !rendered.contains("Read the input, candidate output, and reference before scoring.")
        );
        assert!(rendered.contains("[\"Check factual correctness against the reference.\",\"Penalize unsupported claims.\"]"));
    }

    #[test]
    fn wrapper_builders_reject_empty_or_blank_entries() {
        assert_eq!(
            llm_classifier(
                MockProvider::empty(),
                "judge-model",
                Vec::<String>::new(),
                "Classify the answer.",
            )
            .unwrap_err(),
            LlmJudgeBuildError::EmptyEntries { field: "labels" }
        );

        assert_eq!(
            g_eval(MockProvider::empty(), "judge-model", ["correctness", "   "]).unwrap_err(),
            LlmJudgeBuildError::BlankEntry {
                field: "criteria",
                index: 1,
            }
        );

        assert_eq!(
            llm_classifier_with_labels(
                MockProvider::empty(),
                "judge-model",
                [
                    ClassifierLabel::new("approve"),
                    ClassifierLabel::new("approve").description("duplicate"),
                ],
                "Classify the answer.",
            )
            .unwrap_err(),
            LlmJudgeBuildError::DuplicateEntry {
                field: "labels",
                value: "approve".to_string(),
            }
        );
    }
}
