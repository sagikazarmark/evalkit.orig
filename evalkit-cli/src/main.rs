use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use evalkit_providers::{HttpAcquisition, SubprocessAcquisition, SubprocessScorer};
use evalkit_scorers_text::{contains, exact_match, json_schema};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use evalkit::{
    Acquisition, AcquisitionError, Change, CompareConfig, Comparison, Dataset, Run, RunResult,
    RunStats, Sample, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError, ScorerSet,
    ScorerStats, compare, read_jsonl, write_jsonl,
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
    Diff(DiffArgs),
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

#[derive(clap::Args)]
struct DiffArgs {
    /// Path to the baseline run JSONL file
    #[arg(value_name = "BASELINE")]
    baseline: PathBuf,

    /// Path to the candidate run JSONL file
    #[arg(value_name = "CANDIDATE")]
    candidate: PathBuf,

    /// Optional path to write markdown output
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Optional path to write pretty JSON output
    #[arg(long = "json-output", value_name = "FILE")]
    json_output: Option<PathBuf>,

    /// Confidence level for significance testing
    #[arg(long, default_value_t = 0.95)]
    confidence_level: f64,
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

/// Flat struct — either `url` (HTTP) or `command` (subprocess plugin) must be set, not both.
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
#[derive(Clone, Deserialize)]
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
    command: Option<CommandSpec>,
    pattern: Option<String>,
    schema: Option<Value>,
    timeout_secs: Option<u64>,
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
    Plugin(SubprocessScorer),
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
            CliScorerKind::Plugin(scorer) => scorer.score(ctx).await,
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
        Commands::Diff(args) => match diff_command(args) {
            Ok(()) => 0,
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
    let config: Config =
        toml::from_str(&config_str).map_err(|e| CliError::Config(format!("invalid TOML: {e}")))?;

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

fn diff_command(args: DiffArgs) -> Result<(), CliError> {
    let baseline = load_run_result(&args.baseline)?;
    let candidate = load_run_result(&args.candidate)?;
    let comparison = compare(
        &baseline,
        &candidate,
        CompareConfig {
            confidence_level: args.confidence_level,
        },
    );
    let markdown = render_comparison_markdown(&comparison);

    if let Some(path) = &args.output {
        fs::write(path, markdown.as_bytes())
            .map_err(|e| CliError::Run(format!("cannot write {}: {e}", path.display())))?;
    } else {
        println!("{markdown}");
    }

    if let Some(path) = &args.json_output {
        let file = File::create(path)
            .map_err(|e| CliError::Run(format!("cannot create {}: {e}", path.display())))?;
        serde_json::to_writer_pretty(BufWriter::new(file), &comparison)
            .map_err(|e| CliError::Run(format!("cannot write {}: {e}", path.display())))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_acquisition(cfg: AcquisitionConfig) -> Result<CliAcquisition, CliError> {
    match (cfg.url, cfg.command) {
        (Some(url), None) => Ok(CliAcquisition::Http(
            HttpAcquisition::new(
                url,
                cfg.input_field,
                cfg.output_field,
                Duration::from_secs(cfg.timeout_secs),
            )
            .map_err(|e| CliError::Config(format!("cannot build HTTP client: {e}")))?,
        )),
        (None, Some(cmd)) => {
            let parts = cmd.into_parts();
            if parts.is_empty() {
                return Err(CliError::Config(
                    "[acquisition] command must not be empty".into(),
                ));
            }
            if cfg.input_field != default_input_field()
                || cfg.output_field != default_output_field()
            {
                return Err(CliError::Config(
                    "[acquisition] subprocess plugins always use the canonical `input`/`output` protocol fields".into(),
                ));
            }
            let (program, args) = (parts[0].clone(), parts[1..].to_vec());
            Ok(CliAcquisition::Subprocess(SubprocessAcquisition::new(
                program,
                args,
                Duration::from_secs(cfg.timeout_secs),
            )))
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
        let line =
            line.map_err(|e| CliError::Dataset(format!("read error at line {}: {e}", idx + 1)))?;
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
        samples.push(
            builder.build().map_err(|e| {
                CliError::Dataset(format!("invalid sample at line {}: {e}", idx + 1))
            })?,
        );
    }

    if samples.is_empty() {
        return Err(CliError::Dataset("dataset file contains no samples".into()));
    }

    Ok(samples.into())
}

fn load_run_result(path: &PathBuf) -> Result<RunResult, CliError> {
    let file = File::open(path)
        .map_err(|e| CliError::Run(format!("cannot open {}: {e}", path.display())))?;

    read_jsonl(BufReader::new(file))
        .map_err(|e| CliError::Run(format!("cannot read {}: {e}", path.display())))
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
            let name = entry.name.clone().unwrap_or_else(|| "contains".to_owned());
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
        "plugin" => {
            let command = entry.command.clone().ok_or_else(|| {
                CliError::Config("plugin scorer requires a `command` field".into())
            })?;
            let parts = command.into_parts();
            if parts.is_empty() {
                return Err(CliError::Config(
                    "plugin scorer `command` must not be empty".into(),
                ));
            }

            let name = entry.name.clone().unwrap_or_else(|| "plugin".to_owned());
            let scorer = SubprocessScorer::new(
                parts[0].clone(),
                parts[1..].to_vec(),
                Duration::from_secs(entry.timeout_secs.unwrap_or(default_timeout_secs())),
            );

            Ok(CliScorer {
                definition: ScoreDefinition::new(name),
                kind: CliScorerKind::Plugin(scorer),
            })
        }
        other => Err(CliError::Config(format!(
            "unknown scorer type `{other}`; supported: exact_match, contains, regex, json_schema, plugin"
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

fn render_comparison_markdown(comparison: &Comparison) -> String {
    let mut output = String::new();

    output.push_str("## Eval Diff\n\n");
    output.push_str(&format!("- Baseline: `{}`\n", comparison.baseline_id));
    output.push_str(&format!("- Candidate: `{}`\n", comparison.candidate_id));
    output.push_str(&format!(
        "- Confidence level: `{:.2}`\n\n",
        comparison.confidence_level
    ));

    if !comparison.only_in_baseline.is_empty() {
        output.push_str("### Only In Baseline\n\n");
        for scorer in &comparison.only_in_baseline {
            output.push_str(&format!("- `{scorer}`\n"));
        }
        output.push('\n');
    }

    if !comparison.only_in_candidate.is_empty() {
        output.push_str("### Only In Candidate\n\n");
        for scorer in &comparison.only_in_candidate {
            output.push_str(&format!("- `{scorer}`\n"));
        }
        output.push('\n');
    }

    output.push_str("### Shared Scorers\n\n");
    output.push_str("| Scorer | Aggregate Delta | Significant | Test | Improved | Regressed | Unchanged | Insignificant | Incomparable |\n");
    output.push_str("| --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |\n");

    let mut scorer_names: Vec<_> = comparison.shared_scorers.keys().cloned().collect();
    scorer_names.sort();

    for scorer_name in scorer_names {
        let scorer = comparison
            .shared_scorers
            .get(&scorer_name)
            .expect("shared scorer must exist");
        let counts = change_counts(scorer);
        let significant = match scorer.significant {
            Some(true) => "yes",
            Some(false) => "no",
            None => "n/a",
        };
        let test = scorer.test_used.as_deref().unwrap_or("n/a");

        output.push_str(&format!(
            "| `{}` | {:.4} | {} | `{}` | {} | {} | {} | {} | {} |\n",
            scorer_name,
            scorer.aggregate_delta,
            significant,
            test,
            counts.improved,
            counts.regressed,
            counts.unchanged,
            counts.insignificant,
            counts.incomparable,
        ));
    }

    output
}

struct ChangeCounts {
    improved: usize,
    regressed: usize,
    unchanged: usize,
    insignificant: usize,
    incomparable: usize,
}

fn change_counts(scorer: &evalkit::ScorerComparison) -> ChangeCounts {
    let mut counts = ChangeCounts {
        improved: 0,
        regressed: 0,
        unchanged: 0,
        insignificant: 0,
        incomparable: 0,
    };

    for sample in scorer.sample_comparisons.values() {
        match sample.direction {
            Change::Improved => counts.improved += 1,
            Change::Regressed => counts.regressed += 1,
            Change::Unchanged => counts.unchanged += 1,
            Change::Insignificant => counts.insignificant += 1,
            Change::Incomparable => counts.incomparable += 1,
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use evalkit::{RunMetadata, RunResult, SampleResult, TrialResult};
    use std::time::Duration;

    #[test]
    fn plugin_scorer_requires_a_command() {
        let entry = ScorerConfigEntry {
            scorer_type: String::from("plugin"),
            name: None,
            command: None,
            pattern: None,
            schema: None,
            timeout_secs: None,
        };

        let err = match build_cli_scorer(&entry) {
            Ok(_) => panic!("expected plugin scorer config to fail without a command"),
            Err(err) => err,
        };

        assert_eq!(
            err.to_string(),
            "config error: plugin scorer requires a `command` field"
        );
    }

    #[test]
    fn plugin_scorer_builds_from_command_parts() {
        let entry = ScorerConfigEntry {
            scorer_type: String::from("plugin"),
            name: Some(String::from("external_score")),
            command: Some(CommandSpec::Vec(vec![
                String::from("python3"),
                String::from("score.py"),
            ])),
            pattern: None,
            schema: None,
            timeout_secs: Some(5),
        };

        let scorer = build_cli_scorer(&entry).unwrap();

        assert_eq!(scorer.definition.name, "external_score");
        assert!(matches!(scorer.kind, CliScorerKind::Plugin(_)));
    }

    #[test]
    fn subprocess_acquisition_rejects_custom_protocol_field_names() {
        let err = match build_acquisition(AcquisitionConfig {
            url: None,
            command: Some(CommandSpec::Vec(vec![
                String::from("python3"),
                String::from("plugin.py"),
            ])),
            input_field: String::from("prompt"),
            output_field: String::from("answer"),
            timeout_secs: 30,
        }) {
            Ok(_) => panic!("expected subprocess acquisition config to fail"),
            Err(err) => err,
        };

        assert_eq!(
            err.to_string(),
            "config error: [acquisition] subprocess plugins always use the canonical `input`/`output` protocol fields"
        );
    }

    fn comparison_fixture() -> Comparison {
        let baseline = RunResult {
            metadata: RunMetadata {
                run_id: String::from("baseline"),
                seed: None,
                dataset_fingerprint: String::from("dataset-a"),
                scorer_fingerprint: String::from("scorers-a"),
                code_commit: None,
                code_fingerprint: None,
                judge_model_pins: Vec::new(),
                started_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
                completed_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 5).unwrap(),
                duration: Duration::from_secs(5),
                trial_count: 1,
                score_definitions: vec![ScoreDefinition::maximize("accuracy")],
                acquisition_mode: String::from("inline"),
            },
            samples: vec![SampleResult {
                sample_id: String::from("sample-1"),
                trial_count: 1,
                scored_count: 1,
                error_count: 0,
                token_usage: Default::default(),
                cost_usd: None,
                trials: vec![TrialResult {
                    scores: [(String::from("accuracy"), Ok(Score::Numeric(0.5)))]
                        .into_iter()
                        .collect(),
                    duration: Duration::from_millis(10),
                    trial_index: 0,
                }],
            }],
        };

        let candidate = RunResult {
            metadata: RunMetadata {
                run_id: String::from("candidate"),
                seed: None,
                dataset_fingerprint: String::from("dataset-b"),
                scorer_fingerprint: String::from("scorers-b"),
                code_commit: None,
                code_fingerprint: None,
                judge_model_pins: Vec::new(),
                started_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 1, 0).unwrap(),
                completed_at: Utc.with_ymd_and_hms(2026, 4, 3, 12, 1, 5).unwrap(),
                duration: Duration::from_secs(5),
                trial_count: 1,
                score_definitions: vec![ScoreDefinition::maximize("accuracy")],
                acquisition_mode: String::from("inline"),
            },
            samples: vec![SampleResult {
                sample_id: String::from("sample-1"),
                trial_count: 1,
                scored_count: 1,
                error_count: 0,
                token_usage: Default::default(),
                cost_usd: None,
                trials: vec![TrialResult {
                    scores: [(String::from("accuracy"), Ok(Score::Numeric(0.8)))]
                        .into_iter()
                        .collect(),
                    duration: Duration::from_millis(11),
                    trial_index: 0,
                }],
            }],
        };

        compare(&baseline, &candidate, CompareConfig::default())
    }

    #[test]
    fn render_comparison_markdown_includes_summary_table() {
        let markdown = render_comparison_markdown(&comparison_fixture());

        assert!(markdown.contains("## Eval Diff"));
        assert!(markdown.contains("| Scorer | Aggregate Delta | Significant | Test |"));
        assert!(markdown.contains("`accuracy`"));
        assert!(markdown.contains("| `accuracy` |"));
    }
}
