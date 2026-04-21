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

#[derive(Deserialize)]
struct AcquisitionConfig {
    url: String,
    #[serde(default = "default_input_field")]
    input_field: String,
    #[serde(default = "default_output_field")]
    output_field: String,
    #[serde(default = "default_timeout_secs")]
    timeout_secs: u64,
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

        match payload.get(&self.output_field).and_then(Value::as_str) {
            Some(s) => Ok(s.to_owned()),
            None => Err(AcquisitionError::ExecutionFailed(Box::new(
                MissingOutputField(self.output_field.clone()),
            ))),
        }
    }
}

#[derive(Debug)]
struct MissingOutputField(String);

impl fmt::Display for MissingOutputField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "response JSON is missing the `{}` field", self.0)
    }
}

impl Error for MissingOutputField {}

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
// CLI error type
// ---------------------------------------------------------------------------

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
    let config_str = fs::read_to_string(&args.config).map_err(|e| {
        CliError::Config(format!("cannot read {}: {e}", args.config.display()))
    })?;
    let config: Config = toml::from_str(&config_str)
        .map_err(|e| CliError::Config(format!("invalid TOML: {e}")))?;

    if config.scorers.is_empty() {
        return Err(CliError::Config(
            "config must have at least one [[scorer]] entry".into(),
        ));
    }

    // Load dataset
    let dataset = load_dataset(&args.dataset)?;

    // Build HTTP client and acquisition
    let client = Client::builder()
        .timeout(Duration::from_secs(config.acquisition.timeout_secs))
        .build()
        .map_err(|e| CliError::Config(format!("cannot build HTTP client: {e}")))?;

    let acquisition = HttpAcquisition {
        client,
        url: config.acquisition.url.clone(),
        input_field: config.acquisition.input_field.clone(),
        output_field: config.acquisition.output_field.clone(),
    };

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
                eprintln!(
                    "threshold passed: {scorer_name} = {actual:.4} >= {threshold:.4}"
                );
            }
            Some(actual) => {
                eprintln!(
                    "threshold not met: {scorer_name} = {actual:.4} < {threshold:.4}"
                );
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

fn load_dataset(path: &PathBuf) -> Result<Dataset<String, String>, CliError> {
    let file = File::open(path)
        .map_err(|e| CliError::Dataset(format!("cannot open {}: {e}", path.display())))?;
    let mut samples = Vec::new();

    for (idx, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| {
            CliError::Dataset(format!("read error at line {}: {e}", idx + 1))
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let row: DatasetRow = serde_json::from_str(&line).map_err(|e| {
            CliError::Dataset(format!("invalid JSON at line {}: {e}", idx + 1))
        })?;

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
            let re = Regex::new(pattern).map_err(|e| {
                CliError::Config(format!("invalid regex `{pattern}`: {e}"))
            })?;
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
