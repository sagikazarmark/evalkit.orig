use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;

use evalkit::{
    Acquisition, AcquisitionError, Dataset, Run, RunStats, Sample, Score, ScoreDefinition,
    Scorer, ScorerContext, ScorerError, ScorerSet, ScorerStats, contains, exact_match,
    json_schema, write_jsonl,
};

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "evalkit", about = "Run evals from the command line")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run(RunArgs),
}

#[derive(clap::Args)]
struct RunArgs {
    /// Path to the JSONL dataset file
    #[arg(long, value_name = "FILE")]
    dataset: PathBuf,

    /// Path to the TOML eval config file
    #[arg(long, value_name = "FILE")]
    config: PathBuf,

    /// Optional path to write results JSONL
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Config types (TOML)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Config {
    acquisition: AcquisitionConfig,
    #[serde(default)]
    run: RunConfig,
    #[serde(rename = "scorer", default)]
    scorers: Vec<ScorerConfigEntry>,
    #[serde(default)]
    threshold: HashMap<String, f64>,
}

/// Flat struct — either `url` (HTTP) or `command` (subprocess) must be set, not both.
#[derive(Deserialize)]
struct AcquisitionConfig {
    /// HTTP endpoint URL. Mutually exclusive with `command`.
    url: Option<String>,
    /// Subprocess command. Mutually exclusive with `url`.
    /// A string is split on whitespace; an array is used as-is.
    command: Option<CommandSpec>,
    #[serde(default = "default_input_field")]
    input_field: String,
    #[serde(default = "default_output_field")]
    output_field: String,
    #[serde(default = "default_timeout_secs")]
    timeout_secs: u64,
}

/// `command` can be a plain string (`"python3 model.py"`) or an array
/// (`["python3", "model.py", "--flag"]`). Use the array form when arguments
/// contain spaces.
#[derive(Deserialize)]
#[serde(untagged)]
enum CommandSpec {
    Str(String),
    Vec(Vec<String>),
}

impl CommandSpec {
    fn into_parts(self) -> Vec<String> {
        match self {
            Self::Str(s) => s.split_whitespace().map(str::to_owned).collect(),
            Self::Vec(v) => v,
        }
    }
}

fn default_input_field() -> String {
    "input".to_owned()
}
fn default_output_field() -> String {
    "output".to_owned()
}
fn default_timeout_secs() -> u64 {
    30
}

#[derive(Deserialize, Default)]
struct RunConfig {
    #[serde(default = "default_trials")]
    trials: usize,
    #[serde(default = "default_concurrency")]
    concurrency: usize,
    sample_timeout_secs: Option<u64>,
}

fn default_trials() -> usize {
    1
}
fn default_concurrency() -> usize {
    4
}

#[derive(Deserialize)]
struct ScorerConfigEntry {
    #[serde(rename = "type")]
    scorer_type: String,
    name: Option<String>,
    pattern: Option<String>,
    schema: Option<Value>,
}

// ---------------------------------------------------------------------------
// Dataset JSONL row
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DatasetRow {
    id: Option<String>,
    input: String,
    reference: Option<String>,
}

// ---------------------------------------------------------------------------
// HTTP acquisition
// ---------------------------------------------------------------------------

struct HttpAcquisition {
    client: Client,
    url: String,
    input_field: String,
    output_field: String,
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
            .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;

        let payload: Value = response
            .json()
            .await
            .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;

        extract_string_field(&payload, &self.output_field)
    }
}

// ---------------------------------------------------------------------------
// Subprocess acquisition
//
// Protocol: one JSON line written to stdin, one JSON line read from stdout.
//
//   stdin:  {"<input_field>": "<input text>"}\n
//   stdout: {"<output_field>": "<output text>"}\n
// ---------------------------------------------------------------------------

struct SubprocessAcquisition {
    program: String,
    args: Vec<String>,
    input_field: String,
    output_field: String,
    timeout: Duration,
}

impl Acquisition<String, String> for SubprocessAcquisition {
    async fn acquire(&self, input: &String) -> Result<String, AcquisitionError> {
        tokio::time::timeout(self.timeout, self.run(input))
            .await
            .map_err(|_| AcquisitionError::Timeout(self.timeout))?
    }
}

impl SubprocessAcquisition {
    async fn run(&self, input: &String) -> Result<String, AcquisitionError> {
        let input_json = serde_json::to_string(&json!({ &self.input_field: input }))
            .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;

        let mut child = TokioCommand::new(&self.program)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;

        // Write input JSON line to stdin, then close it so the child sees EOF.
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_json.as_bytes())
                .await
                .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;
        }

        // Read the first line of stdout.
        let stdout = child.stdout.take().expect("stdout was piped");
        let mut reader = TokioBufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;

        // Reap the child process (ignore exit status — a non-zero exit with
        // valid JSON output is still a valid response).
        let _ = child.wait().await;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(AcquisitionError::ExecutionFailed(Box::new(
                EmptyProcessOutput,
            )));
        }

        let payload: Value = serde_json::from_str(trimmed)
            .map_err(|e| AcquisitionError::ExecutionFailed(Box::new(e)))?;

        extract_string_field(&payload, &self.output_field)
    }
}

// ---------------------------------------------------------------------------
// Unified acquisition enum
// ---------------------------------------------------------------------------

enum CliAcquisition {
    Http(HttpAcquisition),
    Subprocess(SubprocessAcquisition),
}

impl Acquisition<String, String> for CliAcquisition {
    async fn acquire(&self, input: &String) -> Result<String, AcquisitionError> {
        match self {
            Self::Http(a) => a.acquire(input).await,
            Self::Subprocess(a) => a.acquire(input).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MissingOutputField(String);

impl fmt::Display for MissingOutputField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "response JSON is missing the `{}` field", self.0)
    }
}

impl Error for MissingOutputField {}

#[derive(Debug)]
struct EmptyProcessOutput;

impl fmt::Display for EmptyProcessOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("subprocess produced no output on stdout")
    }
}

impl Error for EmptyProcessOutput {}

#[derive(Debug)]
enum CliError {
    Config(String),
    Dataset(String),
    Run(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "config error: {msg}"),
            Self::Dataset(msg) => write!(f, "dataset error: {msg}"),
            Self::Run(msg) => write!(f, "run error: {msg}"),
        }
    }
}

impl Error for CliError {}

// ---------------------------------------------------------------------------
// CliScorer — unified enum implementing Scorer<String, String, String>
// ---------------------------------------------------------------------------

struct CliScorer {
    definition: ScoreDefinition,
    kind: CliScorerKind,
}

enum CliScorerKind {
    ExactMatch,
    Contains,
    Regex(Regex),
    JsonSchema(Value),
}

impl Scorer<String, String, String> for CliScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        match &self.kind {
            CliScorerKind::ExactMatch => exact_match().score(ctx).await,
            CliScorerKind::Contains => contains().score(ctx).await,
            CliScorerKind::Regex(re) => Ok(Score::Binary(re.is_match(ctx.output))),
            CliScorerKind::JsonSchema(schema) => {
                let inner_ctx = ScorerContext::<String, String>::new(ctx.input, ctx.output, None);
                json_schema(schema.clone()).score(&inner_ctx).await
            }
        }
    }

    fn definition(&self) -> ScoreDefinition {
        self.definition.clone()
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::Run(args) => match run_command(args).await {
            Ok(true) => 0,
            Ok(false) => 1,
            Err(e) => {
                eprintln!("error: {e}");
                2
            }
        },
    };
    std::process::exit(exit_code);
}

async fn run_command(args: RunArgs) -> Result<bool, CliError> {
    // Read and parse config
    let config_str = fs::read_to_string(&args.config)
        .map_err(|e| CliError::Config(format!("cannot read {}: {e}", args.config.display())))?;
    let config: Config = toml::from_str(&config_str)
        .map_err(|e| CliError::Config(format!("invalid TOML: {e}")))?;

    if config.scorers.is_empty() {
        return Err(CliError::Config(
            "config must have at least one [[scorer]] entry".into(),
        ));
    }

    // Load dataset
    let dataset = load_dataset(&args.dataset)?;

    // Build acquisition
    let acquisition = build_acquisition(config.acquisition)?;

    // Build scorers and scorer set
    let cli_scorers = build_cli_scorers(&config.scorers)?;
    let mut scorer_iter = cli_scorers.into_iter();
    let first = scorer_iter.next().unwrap(); // safe: checked is_empty above
    let mut scorer_builder = ScorerSet::<String, String, String>::builder().scorer(first);
    for s in scorer_iter {
        scorer_builder = scorer_builder.scorer(s);
    }
    let scorer_set = scorer_builder.build();

    // Build run
    let mut run_builder = Run::builder()
        .dataset(dataset)
        .acquisition(acquisition)
        .scorer_set(scorer_set)
        .trials(config.run.trials)
        .concurrency(config.run.concurrency);

    if let Some(secs) = config.run.sample_timeout_secs {
        run_builder = run_builder.sample_timeout(Duration::from_secs(secs));
    }

    let run = run_builder
        .build()
        .map_err(|e| CliError::Run(e.to_string()))?;

    // Execute
    eprintln!("Running eval...");
    let result = run
        .execute()
        .await
        .map_err(|e| CliError::Run(format!("execution failed: {e}")))?;

    // Write results JSONL if requested
    if let Some(output_path) = &args.output {
        let file = File::create(output_path)
            .map_err(|e| CliError::Run(format!("cannot create output file: {e}")))?;
        write_jsonl(&result, BufWriter::new(file))
            .map_err(|e| CliError::Run(format!("cannot write results: {e}")))?;
        eprintln!("Results written to {}", output_path.display());
    }

    // Print stats
    let stats = result.stats();
    eprintln!("{}", stats.summary());

    // Check thresholds
    if config.threshold.is_empty() {
        return Ok(true);
    }

    let mut all_passed = true;
    for (scorer_name, &threshold) in &config.threshold {
        match primary_value(&stats, scorer_name) {
            Some(actual) if actual >= threshold => {
                eprintln!("threshold passed: {scorer_name} = {actual:.4} >= {threshold:.4}");
            }
            Some(actual) => {
                eprintln!("threshold not met: {scorer_name} = {actual:.4} < {threshold:.4}");
                all_passed = false;
            }
            None => {
                eprintln!(
                    "warning: no numeric stats for scorer '{scorer_name}', skipping threshold check"
                );
            }
        }
    }

    Ok(all_passed)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_acquisition(cfg: AcquisitionConfig) -> Result<CliAcquisition, CliError> {
    match (cfg.url, cfg.command) {
        (Some(url), None) => {
            let client = Client::builder()
                .timeout(Duration::from_secs(cfg.timeout_secs))
                .build()
                .map_err(|e| CliError::Config(format!("cannot build HTTP client: {e}")))?;
            Ok(CliAcquisition::Http(HttpAcquisition {
                client,
                url,
                input_field: cfg.input_field,
                output_field: cfg.output_field,
            }))
        }
        (None, Some(cmd)) => {
            let parts = cmd.into_parts();
            if parts.is_empty() {
                return Err(CliError::Config(
                    "[acquisition] command must not be empty".into(),
                ));
            }
            let (program, args) = (parts[0].clone(), parts[1..].to_vec());
            Ok(CliAcquisition::Subprocess(SubprocessAcquisition {
                program,
                args,
                input_field: cfg.input_field,
                output_field: cfg.output_field,
                timeout: Duration::from_secs(cfg.timeout_secs),
            }))
        }
        (Some(_), Some(_)) => Err(CliError::Config(
            "[acquisition] specifies both `url` and `command`; choose one".into(),
        )),
        (None, None) => Err(CliError::Config(
            "[acquisition] requires either `url` (HTTP) or `command` (subprocess)".into(),
        )),
    }
}

fn load_dataset(path: &PathBuf) -> Result<Dataset<String, String>, CliError> {
    let file = File::open(path)
        .map_err(|e| CliError::Dataset(format!("cannot open {}: {e}", path.display())))?;
    let mut samples = Vec::new();

    for (idx, line) in BufReader::new(file).lines().enumerate() {
        let line = line
            .map_err(|e| CliError::Dataset(format!("read error at line {}: {e}", idx + 1)))?;
        if line.trim().is_empty() {
            continue;
        }
        let row: DatasetRow = serde_json::from_str(&line)
            .map_err(|e| CliError::Dataset(format!("invalid JSON at line {}: {e}", idx + 1)))?;

        let mut builder = Sample::<String, String>::builder(row.input);
        if let Some(id) = row.id {
            builder = builder.id(id);
        }
        if let Some(reference) = row.reference {
            builder = builder.reference(reference);
        }
        samples.push(builder.build().map_err(|e| {
            CliError::Dataset(format!("invalid sample at line {}: {e}", idx + 1))
        })?);
    }

    if samples.is_empty() {
        return Err(CliError::Dataset("dataset file contains no samples".into()));
    }

    Ok(samples.into())
}

fn build_cli_scorers(entries: &[ScorerConfigEntry]) -> Result<Vec<CliScorer>, CliError> {
    entries.iter().map(build_cli_scorer).collect()
}

fn build_cli_scorer(entry: &ScorerConfigEntry) -> Result<CliScorer, CliError> {
    match entry.scorer_type.as_str() {
        "exact_match" => {
            let name = entry
                .name
                .clone()
                .unwrap_or_else(|| "exact_match".to_owned());
            Ok(CliScorer {
                definition: ScoreDefinition::new(name),
                kind: CliScorerKind::ExactMatch,
            })
        }
        "contains" => {
            let name = entry
                .name
                .clone()
                .unwrap_or_else(|| "contains".to_owned());
            Ok(CliScorer {
                definition: ScoreDefinition::new(name),
                kind: CliScorerKind::Contains,
            })
        }
        "regex" => {
            let pattern = entry.pattern.as_deref().ok_or_else(|| {
                CliError::Config("regex scorer requires a `pattern` field".into())
            })?;
            let re = Regex::new(pattern)
                .map_err(|e| CliError::Config(format!("invalid regex `{pattern}`: {e}")))?;
            let name = entry.name.clone().unwrap_or_else(|| "regex".to_owned());
            Ok(CliScorer {
                definition: ScoreDefinition::new(name),
                kind: CliScorerKind::Regex(re),
            })
        }
        "json_schema" => {
            let schema = entry.schema.clone().ok_or_else(|| {
                CliError::Config("json_schema scorer requires a `schema` field".into())
            })?;
            let name = entry
                .name
                .clone()
                .unwrap_or_else(|| "json_schema".to_owned());
            Ok(CliScorer {
                definition: ScoreDefinition::new(name),
                kind: CliScorerKind::JsonSchema(schema),
            })
        }
        other => Err(CliError::Config(format!(
            "unknown scorer type `{other}`; supported: exact_match, contains, regex, json_schema"
        ))),
    }
}

/// Returns the primary numeric value for a scorer (pass_rate for binary, mean otherwise).
fn primary_value(stats: &RunStats, scorer_name: &str) -> Option<f64> {
    match stats.scorer_stats.get(scorer_name)? {
        ScorerStats::Binary { pass_rate, .. } => Some(*pass_rate),
        ScorerStats::Numeric { mean, .. } | ScorerStats::Metric { mean, .. } => Some(*mean),
        ScorerStats::Label { .. } => None,
    }
}

fn extract_string_field(payload: &Value, field: &str) -> Result<String, AcquisitionError> {
    match payload.get(field).and_then(Value::as_str) {
        Some(s) => Ok(s.to_owned()),
        None => Err(AcquisitionError::ExecutionFailed(Box::new(
            MissingOutputField(field.to_owned()),
        ))),
    }
}
