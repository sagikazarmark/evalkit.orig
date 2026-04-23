> **📦 Archived on 2026-04-23** — superseded by [Glossary (Replacement for Section 14)](../../../docs/spec/glossary-rewrite.md). Kept for historical reference.

# Glossary (Replacement for Section 14)

## Core Concepts

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `evaluation` / `eval` | The process of measuring an AI system's quality by giving it inputs and scoring its outputs. | Aligned — "eval" is universal shorthand (Anthropic, DeepEval, Inspect AI, all major tools). | |
| `Sample` | A single evaluation data point: an input to the system under evaluation, plus an optional reference value for comparison. Identified by a stable `id` (content-hashed by default). | Fractured — "Task" (Inspect AI, Anthropic), "Test Case" (DeepEval), "Sample" (RAGAS), "Problem", "Scenario". | Deliberate choice: neutral, non-test-framework, avoids Rust keyword conflicts (`test`, `case`). Follows RAGAS convention. |
| `Dataset` | An ordered collection of Samples. | Aligned — "Dataset" is universal across the domain. | Some tools distinguish "Dataset" (data only) from "Test Suite" (data + assertions). This project uses Dataset for both since assertions live in Scorers, not in data. |
| `reference` | The expected or comparison value in a Sample. Used by scorers that compare the system's output against a known-good value. `Option<R>` — not all evaluations need one. | Fractured — "Ground Truth" (Microsoft, AWS), "Golden Answer", "Reference" (RAGAS), "Target" (Inspect AI), "Expected Output" (DeepEval), "Expected" (Braintrust, Promptfoo). | Deliberate choice: "reference" is neutral about correctness — many evaluations have no single correct answer, only a reference point. Follows RAGAS convention. |

## Scoring

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Scorer` | A component that produces a Score from a ScorerContext (input, output, and optional reference). Implements the `Scorer<I, O, R>` trait. | Fractured — "Grader" (Anthropic, OpenAI), "Scorer" (Inspect AI, Braintrust), "Evaluator" (LangChain, Arize, OpenEvals), "Metric" (DeepEval, RAGAS), "Assertion" (Promptfoo). | Deliberate choice: most neutral term, pairs cleanly with Score, avoids "Evaluator" (which collides with the evaluation process itself). Follows Inspect AI and Braintrust convention. This is the domain's single most fragmented term. |
| `Score` | The typed output of a Scorer. An enum with four variants: Numeric, Binary, Label, Metric. | Aligned — "Score" is universal when paired with "Scorer". | Broader than everyday "score" — includes boolean pass/fail and categorical classifications, not just numbers. |
| `Numeric` | Score variant: a continuous numeric value (`f64`). Convention is 0.0–1.0 but not enforced. | No standard variant name exists in the domain. | Novel term. Chosen over "Continuous" for developer readability. |
| `Binary` | Score variant: pass/fail (`bool`). | Aligned with the universal pass/fail concept. | |
| `Label` | Score variant: a categorical classification (`String`). | No standard variant name. | Novel term. Self-explanatory for categorical data. |
| `Metric` | Score variant: a named measurement with a numeric value and optional unit. E.g., `Metric { name: "latency_ms", value: 230.0, unit: Some("ms") }`. | Aligned — "Metric" is the standard English word for a named measurement. | **⚠️ Domain collision:** DeepEval and RAGAS use "Metric" to mean the scorer component itself (`FaithfulnessMetric`, `PlanQualityMetric`). In this project, Metric is a **Score variant** (a measurement value), not a Scorer. A DeepEval `Metric` is this project's `Scorer`; a DeepEval metric's output is this project's `Score::Metric`. |
| `ScoreDefinition` | Defines what a score means: a name and an optional direction. Returned by `Scorer::definition()`. Persisted in RunMetadata. | No standard — no other tool formalizes this as a separate type. | Novel concept. Analogous to defining a unit of measurement before taking measurements. Extensible (can add description, value range in future). |
| `Direction` | Whether a higher or lower score is better: `Maximize` or `Minimize`. `Option<Direction>` — `None` for Binary and Label scores where the concept doesn't apply. | No standard formalization. Concept is implicit in every metric (accuracy = maximize, latency = minimize). | Novel formalization of an implicit domain concept. |
| `ScorerContext` | The input bundle passed to a Scorer: `input`, `output`, and optional `reference`. Marked `#[non_exhaustive]` for future extension (e.g., metadata, trial info, cost). | No standard. Inspect AI passes individual params; DeepEval uses a test case object. | Novel. Signals extensibility; avoids ambiguity with "input" (which means the sample's input). |
| `ScorerError` | A scorer infrastructure failure — distinct from a low score. A network error reaching an LLM judge, a regex compilation failure, or a mapper error are ScorerErrors, not findings. | Aligned concept — DeepEval: `metric.error`, Inspect AI: `EvalError`, Promptfoo: `ResultFailureReason.ERROR`. All major frameworks distinguish errors from low scores. | Name follows this project's "Scorer" naming convention. |
| `ScorerSet` | A group of scorers that share output and/or reference mappers. The mapper runs once per trial; its result is shared by all scorers in the set. | No standard — novel concept. | Reduces redundancy when multiple scorers need the same transformation. E.g., a set of scorers that all need JSON parsed from a string output. |

## Mapping & Transformation

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Mapper` | A type conversion trait used for both output mapping (`O → O2`) and reference mapping (`R → R2`). Closures implement it automatically via blanket impl. | No domain equivalent in eval tools. DeepEval has implicit preprocessing. | Novel in the eval domain. Familiar from `Iterator::map` in Rust and functional programming. Unifies what would be separate "transform" and "reference converter" concepts under one trait. |
| `MapError` | Error from a Mapper (output or reference mapping). When a mapper in a ScorerSet fails, all scorers in that set receive this as a ScorerError. | No standard. | Follows from Mapper. |

## Acquisition

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Acquisition` | How outputs are obtained for evaluation. The architectural boundary between "get the output" and "score it." A trait with two built-in implementations: inline (closure) and observe (OTel traces). | No domain equivalent — every other eval tool either assumes inline execution or (agentevals-dev) evaluates from traces without formalizing the abstraction. | Novel. Names a previously-unnamed concept. In inline mode, acquisition calls a function and receives its return value. In observe mode, acquisition queries a trace backend and returns spans. |
| `inline` | Acquisition mode: the framework calls a user-provided function (closure) to get the output. The default mode and the way most eval tools work. | No standard — this is the implicit default in all eval tools, but no tool names it. | Novel. |
| `observe` | Acquisition mode: the framework queries existing OTel traces to extract outputs. The agent runs externally; the framework only reads spans. Enables evaluation without re-execution. | No standard. agentevals-dev does this but doesn't name the mode. | Novel. The framework does NOT call the agent — it only reads traces. Historical and live traces are the same operation. |
| `correlation_id` | A domain-level identifier (e.g., a run ID or execution ID) used by observe-mode to query a TraceBackend for matching spans. The framework groups returned spans per sample using the `sample_attribute`. | Novel — no standard equivalent. OTel has "trace context" for distributed tracing propagation, but correlation here is done via OTel baggage/attributes, not trace IDs. | An agent execution may span multiple OTel traces. Correlation by domain-level ID (via baggage attributes like `eval.run_id`) is more robust than correlation by trace ID. |
| `sample_attribute` | The name of the OTel span attribute used to group spans per sample in observe mode. E.g., `"eval.sample_id"`. | No standard. | Configurable per Observe builder. The TraceBackend groups spans by this attribute's value, matching them to Sample IDs. |
| `AcquisitionError` | Error from obtaining output for a sample. Variants: `ExecutionFailed` (function error/panic), `TraceNotFound` (no matching spans), `BackendUnavailable` (trace backend unreachable), `Timeout` (exceeded sample_timeout). | No standard. | Per-sample, per-trial error. Other samples/trials continue. |

## OTel / Observability

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Trace` | An OpenTelemetry execution record — a tree of spans representing one request or operation through a system. | Aligned — "Trace" is the OTel standard term, universal in observability. | In evaluation context, traces contain the evidence of what an agent did. Distinct from Trajectory (see below). |
| `Span` | An individual operation within a trace: has a trace ID, span ID, operation name, timestamps, attributes, and events. | Aligned — "Span" is the OTel standard term. | Represented as a struct with `attributes: HashMap<String, Value>` for flexible access. |
| `SpanEvent` | An event within a Span: a named, timestamped occurrence with attributes. | Aligned — OTel standard concept (Span Events). | |
| `Trajectory` | The evaluation-relevant sequence of agent decisions and actions. Extracted from a Trace for evaluation purposes. | Aligned — "Trajectory" (LangChain AgentEvals, Anthropic, Arize). | Distinct from Trace: a trace is an observability record; a trajectory is the subset relevant to evaluating agent behavior. In this framework, trajectory extraction is done via a Mapper on `Vec<Span>`. |
| `TraceBackend` | A pluggable source for querying OTel spans. Given a correlation ID and sample attribute, returns matching spans grouped by sample. | No standard. | Novel. Built-in: `JaegerBackend` (Jaeger v2 API). Future: Tempo, OTLP receiver. Users can implement custom backends. |
| `TraceBackendError` | Error from a TraceBackend query (e.g., backend unreachable, query timeout, malformed response). | No standard. | Follows from TraceBackend. Wrapped into `AcquisitionError::BackendUnavailable` at the acquisition layer. |

## Evaluation Execution

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Run` | A complete evaluation execution: dataset × acquisition × mappers × scorers × trials. Built via a builder pattern. | Aligned — "Run" is the common informal term in the domain. | Braintrust uses "Experiment" for a similar concept. The builder enforces required components and type correctness. `.execute()` is the verb (avoids `run.run()`). |
| `Trial` | One execution attempt of one sample: acquire output → map → score. In multi-trial mode, each sample runs N trials to account for non-determinism. | Aligned — "Trial" (Anthropic, Agentrial). More formal than "Run" or "Attempt". | Trial is a runner concept, not a scorer concept. Scorers are pure functions; the Run manages trial repetition. |
| `LLM-as-a-Judge` | Evaluation methodology where an LLM scores another system's output based on a prompt template. The judge LLM receives the input, output, and optionally the reference, and returns a Score. | Aligned — "LLM-as-a-Judge" is the settled domain term (Wikipedia-notable). Used by virtually every eval tool (Langfuse, Arize, DeepEval, Inspect AI). | In this project: `llm_judge()` function (Rust snake_case), `llm-judge` feature gate (Cargo kebab-case). "Model-graded" is a legacy synonym (OpenAI). |
| `pass_at_k` | At least one pass in k trials. Standard metric for non-deterministic evaluation: "did the system succeed at least once?" | Aligned — "pass@k" (HumanEval paper, Chen et al. 2021; Anthropic; EvalPlus). Settled term. | Underscore form is a Rust casing adaptation of "pass@k". |
| `pass_all_k` | All k trials pass. Stricter metric: "does the system succeed every time?" | Aligned — "pass^k" (Anthropic). | More readable than "pass^k". |

## Results

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `RunResult` | The raw output of a completed Run: all trial results for all samples, plus metadata. Contains no computed statistics — stats are derived separately via `.stats()`. | No standard as a distinct raw-data concept. Most tools conflate raw results with computed stats. | Novel. The separation of raw data from interpretation is a deliberate design choice (decisions-log Round 3). |
| `TrialResult` | The result of one trial for one sample: a score (or error) per scorer, plus wall-clock duration. | No standard. | Keyed by scorer name. A trial with partial scorer success (some scorers scored, some errored) counts as scored. |
| `SampleResult` | All trial results for one sample, plus aggregate counts: `trial_count`, `scored_count`, `error_count`. | No standard. | `scored_count` = trials with at least one successful score. `error_count` = trials where ALL scorers errored. |
| `RunMetadata` | Execution metadata for a completed Run: unique ID, timestamps, duration, trial count, scorer definitions, and acquisition mode ("inline" or "observe"). | No standard. | Auto-generated UUID for `run_id`. `score_definitions` persists each scorer's ScoreDefinition for downstream comparison. |

## Statistics

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `RunStats` | Computed statistics from a RunResult: per-scorer aggregated stats across all samples. Produced by `RunResult::stats()`. | No standard as a separate concept. | Novel. Computed, not stored in RunResult. |
| `ScorerStats` | Per-score-type aggregated statistics. An enum matching Score variants: Numeric (mean, stddev, CI, min, max), Binary (pass_rate, pass_at_k, pass_all_k, CI), Label (distribution, mode), Metric (same shape as Numeric). | No standard. | Novel. Errored trials excluded from denominator and reported separately. Wilson CI for Binary; t-distribution CI for Numeric/Metric. |

## Comparison

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `baseline` | The known-good reference run in a comparison. | Aligned — "baseline" (criterion, benchstat). Standard in benchmarking and A/B testing. | |
| `candidate` | The run under evaluation in a comparison. | Common in A/B testing and release engineering. | Most benchmark tools don't name the second side; "candidate" is more explicit than "new" or "test". |
| `Comparison` | The result of comparing two RunResults: per-scorer deltas, significance tests, and per-sample Change directions. | No standard as a distinct type. | Novel. Reports shared scorers, scorers only in baseline, and scorers only in candidate. |
| `ScorerComparison` | Per-scorer comparison within a Comparison: sample-level deltas, aggregate delta (candidate − baseline), p-value, significance flag, and name of statistical test used. | No standard. | Direction-aware: respects ScoreDefinition.direction. |
| `SampleComparison` | Per-sample comparison within a ScorerComparison: score delta and Change direction. | No standard. | |
| `Change` | The direction of difference between baseline and candidate for one sample or aggregate. Variants: `Improved` (candidate is better, direction-aware), `Regressed` (candidate is worse), `Unchanged` (no difference), `Insignificant` (delta exists but not statistically significant), `Incomparable` (different score types, direction mismatch, or insufficient data). | No standard formalization. | Novel. `Incomparable` is a safety variant — prevents misleading comparisons when runs have different configurations. |

## Errors

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `RunBuildError` | Error from invalid Run builder configuration (pre-execution). Variants: `NoDataset`, `NoAcquisition`, `NoScorer`, `EmptyDataset`, `DuplicateSampleIds`, `DuplicateScorerNames`, `MissingSampleIds`. | No standard. | `#[non_exhaustive]`. All caught before any evaluation runs. |
| `RunError` | Error preventing a Run from producing any result. Variants: `Build(RunBuildError)` and `Internal(...)`. | No standard. | `#[non_exhaustive]`. `execute()` returns `Ok(RunResult)` for all evaluation outcomes, including failures. `Err(RunError)` only when no result is possible at all. |
