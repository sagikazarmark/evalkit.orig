# Technical Specification: The Eval Kernel

## Meta
- **Date**: 2026-04-03
- **Status**: Draft
- **Direction source**: brainstorm/directions.md → Direction 1 (Eval Kernel) + Direction 2 (Trace Grader) + Direction 3 (Confident Eval)
- **Research corpus version**: 2026-04-02

---

## 1. Problem Statement

AI agent evaluation is fragmented across 65+ tools, none of which provides a generic, typed evaluation foundation in Rust (research/01-landscape.md — zero general-purpose Rust eval frameworks exist). Every existing tool bakes AI-specific concepts into its core API, forcing users into opinionated execution models and preventing reuse across diverse evaluation scenarios (research/05-pain-points.md — lock-in is a documented pain point across DeepEval, LangSmith, and Promptfoo). Meanwhile, the #1 gap in the domain — statistical treatment of non-deterministic agent evaluation — remains unaddressed by any mainstream tool (research/synthesis.md, Gap 1: Critical unmet need; only Agentrial at 16 stars attempts this). This specification defines a Rust crate providing typed evaluation primitives with a domain-agnostic core, multiple acquisition modes (inline execution and OTel trace observation), and first-class multi-trial statistical aggregation — a combination that exists nowhere in the current landscape.

---

## 2. Scope

### In Scope
- Core evaluation types: Sample, Dataset, Score, Scorer trait, ScorerContext, ScoreDefinition
- Mapper trait for type-safe output and reference transformations
- ScorerSet for grouping scorers with shared mappers
- Acquisition trait with inline (closure) and observe (OTel trace) implementations
- TraceBackend trait with Jaeger v2 API implementation
- Run builder with sequential execution
- Multi-trial execution with configurable trial count
- RunResult with per-sample, per-trial, per-scorer results
- Statistical aggregation: mean, stddev, CI (Wilson for proportions, t-distribution for continuous), pass_at_k, pass_all_k
- Run-to-run comparison with significance testing (t-test, Fisher's exact test)
- Built-in scorers: exact_match, contains, regex, json_schema
- LLM-as-a-Judge scorer (feature-gated)
- JSONL convenience serialization
- serde Serialize/Deserialize on all public types

### Out of Scope
- **Platform features** — no dashboards, cloud service, user accounts, or storage system (brainstorm non-goal; research/09-failure-archaeology.md: premature platform-building is a documented failure pattern)
- **Observability** — does not collect traces or replace Langfuse/Jaeger (non-goal)
- **Scorer catalog** — 4 built-in scorers + LLM-as-a-Judge; differentiation is the framework, not pre-built scorers (non-goal)
- **Python bindings** — Rust-first; API designed to not prevent future PyO3 wrapping (deferred; braindump T-02)
- **CLI** — library only; CLI is a future higher-level component (decisions-log: CLI explicitly dropped during brainstorm)
- **Concurrent execution** — sequential MVP; `.concurrency(N)` API designed but implementation deferred (decisions-log Round 4)
- **Scorer composition** — `.and()`, `.weighted()`, `.then()` designed but deferred to post-MVP (decisions-log Round 4)
- **Cost tracking** — deferred to growth path
- **Drift detection** — CUSUM/Page-Hinkley deferred to growth path
- **OTLP receiver** — future TraceBackend implementation (deferred)
- **Failure threshold / abort on error rate** — Inspect AI pattern researched, deferred until large datasets create the need (decisions-log Round 6)

---

## 3. User Stories & Acceptance Criteria

### US-01: Score a text output against a reference value

As a Rust developer evaluating an AI agent,
I want to score an agent's text output against a reference value using a built-in scorer,
so that I can quickly check whether my agent produces correct results.

Acceptance criteria:
- [ ] AC-01.1: Can construct a `Sample` with an input string and a reference string
- [ ] AC-01.2: Can call `exact_match` scorer with the sample and an actual output, and receive a `Score::Binary`
- [ ] AC-01.3: Can call `contains` scorer and receive a `Score::Binary`
- [ ] AC-01.4: Can call `regex` scorer with a pattern and receive a `Score::Binary`
- [ ] AC-01.5: All scorers return `Result<Score, ScorerError>`, not bare `Score`
- [ ] AC-01.6: Scorer errors (e.g., invalid regex pattern) are distinguishable from low scores

### US-02: Run an evaluation across a dataset

As a Rust developer with a set of test cases,
I want to run a scorer across an entire dataset and get structured results,
so that I can see aggregate performance, not just individual scores.

Acceptance criteria:
- [ ] AC-02.1: Can construct a `Dataset` from a `Vec<Sample>`
- [ ] AC-02.2: Can build a `Run` with a dataset, an acquisition (closure), and one or more scorers
- [ ] AC-02.3: `Run::execute()` returns a `RunResult` containing a `SampleResult` per sample
- [ ] AC-02.4: Each `SampleResult` contains the scores from each scorer per trial
- [ ] AC-02.5: Execution is async
- [ ] AC-02.6: Run accepts multiple scorers and/or ScorerSets

### US-03: Serialize and compare results

As a developer running evaluations in CI,
I want to serialize a `RunResult` to JSONL and compare two results,
so that I can detect regressions between branches.

Acceptance criteria:
- [ ] AC-03.1: `RunResult` serializes to JSONL via convenience functions
- [ ] AC-03.2: `RunResult` deserializes from JSONL back to a typed struct
- [ ] AC-03.3: Can load two `RunResult`s and produce a comparison showing per-sample score deltas
- [ ] AC-03.4: Comparison output indicates which samples improved, regressed, or stayed the same
- [ ] AC-03.5: Comparison respects `ScoreDefinition.direction` — lower latency is an improvement for `Minimize` scorers

### US-04: Compose scorers declaratively (DEFERRED)

As a developer with complex evaluation criteria,
I want to combine multiple scorers using composition operators,
so that I can express evaluation logic as a single declarative pipeline.

Acceptance criteria:
- [ ] AC-04.1: `.and(other)` runs both scorers and returns a combined score
- [ ] AC-04.2: `.weighted(other, w1, w2)` returns a weighted average of two Numeric scores
- [ ] AC-04.3: `.then(other)` runs the second scorer only if the first passes (short-circuit)
- [ ] AC-04.4: Composed scorers implement the same `Scorer` trait — composition is recursive
- [ ] AC-04.5: When a composed scorer fails, the error identifies which sub-scorer failed

**Status: Deferred to post-MVP. Pure library code, addable without changing other components.**

### US-05: Score with an LLM-as-a-Judge

As a developer evaluating subjective output quality,
I want to use an LLM as a scorer,
so that I can evaluate outputs where deterministic checks are insufficient.

Acceptance criteria:
- [ ] AC-05.1: Can configure an LLM-as-a-Judge scorer with a prompt template and score extraction logic
- [ ] AC-05.2: The scorer sends the input, output, and optionally reference to the LLM and parses the response into a `Score`
- [ ] AC-05.3: Network errors and parse errors return `ScorerError`, not a low score
- [ ] AC-05.4: The scorer is async
- [ ] AC-05.5: The scorer is behind a feature gate (`llm-judge`)

### US-06: Use ScorerSets with shared mappers

As a developer evaluating complex output types,
I want to group scorers that share a transformation and apply it once,
so that expensive transforms run once and scorers see the right data types.

Acceptance criteria:
- [ ] AC-06.1: Can create a `ScorerSet` with `.map_output()` and one or more scorers
- [ ] AC-06.2: Can create a `ScorerSet` with `.map_reference()` to transform the reference value
- [ ] AC-06.3: The mapper runs once per trial; its result is shared by all scorers in the set
- [ ] AC-06.4: Mapper errors are captured as `ScorerError` for all scorers in the set
- [ ] AC-06.5: Scorers outside ScorerSets see the post-global-map output (or raw acquisition output if no global mapper)

### US-07: Evaluate an agent from OTel traces

As a developer with OTel-instrumented agents,
I want to point the framework at a set of spans identified by a correlation ID and have it extract outputs and score them,
so that I can evaluate without the framework needing to know how to call my agent.

Acceptance criteria:
- [ ] AC-07.1: Can configure a Run with observe-mode acquisition
- [ ] AC-07.2: Observe-mode accepts a correlation identifier (e.g., a run/execution ID)
- [ ] AC-07.3: The framework queries a trace backend for spans matching the correlation identifier
- [ ] AC-07.4: Spans are grouped per sample using a sample-level attribute (e.g., `eval.sample_id`)
- [ ] AC-07.5: Output extraction is done via Mapper on `Vec<Span>` — same Mapper trait as everywhere else
- [ ] AC-07.6: The extracted output is passed to scorers — scoring is identical to inline mode
- [ ] AC-07.7: Configurable timeout/retry for span collection
- [ ] AC-07.8: If no spans match a sample, the sample result is a collection error, not a low score
- [ ] AC-07.9: Observe-mode is behind a feature gate (`otel`)
- [ ] AC-07.10: Historical traces and live traces are the same operation — no separate "replay" mode needed

*Note: US-08 (custom span extraction) absorbed into US-07 — extraction is a Mapper on `Vec<Span>`. US-12 (historical trace eval) merged into US-07 — same operation.*

### US-09: Use different trace sources

As a developer,
I want to swap the trace source without changing my evaluation logic,
so that the framework adapts to my infrastructure.

Acceptance criteria:
- [ ] AC-09.1: `TraceBackend` is a trait the user can implement
- [ ] AC-09.2: Built-in implementation for Jaeger v2 API
- [ ] AC-09.3: The trait contract: given a correlation ID → return matching spans, grouped by sample attribute
- [ ] AC-09.4: Future OTLP receiver would be another TraceBackend implementation (out of scope)

### US-10: Run multi-trial evaluations with statistical aggregation

As a developer evaluating a non-deterministic agent,
I want to run each sample multiple times and see statistically rigorous results,
so that I know whether score differences are real or noise.

Acceptance criteria:
- [ ] AC-10.1: `.trials(N)` on the Run builder runs each sample N times
- [ ] AC-10.2: Per-sample stats include: mean, standard deviation for Numeric/Metric scores
- [ ] AC-10.3: Per-sample stats include: pass_rate, `pass_at_k`, `pass_all_k` for Binary scores
- [ ] AC-10.4: Per-sample stats include: label distribution and mode for Label scores
- [ ] AC-10.5: Confidence intervals: Wilson CI for Binary, t-distribution CI for Numeric/Metric
- [ ] AC-10.6: Aggregate stats across all samples per scorer
- [ ] AC-10.7: Single-trial runs (`.trials(1)` or omitted) produce results in the same structure — no special-casing

### US-11: Compare two runs with significance testing

As a developer comparing a code change,
I want to know whether the difference between two evaluation runs is statistically significant,
so that I don't roll back changes based on noise.

Acceptance criteria:
- [ ] AC-11.1: Can compare two `RunResult`s (baseline vs candidate)
- [ ] AC-11.2: For Binary scores: appropriate significance test and p-value
- [ ] AC-11.3: For Numeric scores: t-test or equivalent and p-value
- [ ] AC-11.4: Reports whether the difference is significant at a configurable confidence level (default 0.95)
- [ ] AC-11.5: Comparison handles runs with different trial counts (approach TBD — see OQ-05)
- [ ] AC-11.6: Direction-aware: respects `ScoreDefinition.direction` (Maximize vs Minimize)
- [ ] AC-11.7: Direction mismatch between baseline and candidate → `Change::Incomparable`

### US-13: Scorer failure is not a low score

As a developer debugging evaluation infrastructure,
I want scorer errors to be clearly distinct from low scores,
so that infrastructure problems don't silently look like quality problems.

Acceptance criteria:
- [ ] AC-13.1: A scorer that returns `Err(ScorerError)` produces a result with error status, not a `Score` of 0
- [ ] AC-13.2: Aggregate statistics exclude errored trials from the denominator and report error count separately
- [ ] AC-13.3: The `RunResult` distinguishes between "scored and low" vs. "failed to score"

---

## 4. System Architecture

### 4.1 Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Run Builder                          │
│  Orchestrates: dataset × acquisition × mappers × scorers     │
│  Manages: trials, error collection                           │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │  Acquisition  │    │   Mapper      │    │  Scorer        │ │
│  │  (trait)      │    │   (trait)     │    │  (trait)       │ │
│  │  ┌─────────┐ │    │  map_output  │    │  ScorerSet    │  │
│  │  │ Fn impl │ │    │  map_ref     │    │               │  │
│  │  ├─────────┤ │    └──────────────┘    └───────────────┘  │
│  │  │ Observe │ │                                           │
│  │  │ (otel)  │ │                                           │
│  │  └─────────┘ │                                           │
│  └──────────────┘                                           │
├─────────────────────────────────────────────────────────────┤
│                        Core Types                            │
│  Sample, Dataset, Score, ScoreDefinition, ScorerContext,     │
│  ScorerError, Direction, Mapper                              │
├─────────────────────────────────────────────────────────────┤
│                        Results                               │
│  RunResult, SampleResult, TrialResult, RunMetadata           │
├─────────────────────────────────────────────────────────────┤
│                     Stats (derived)                          │
│  ScorerStats, AggregateStats — computed from RunResult       │
├─────────────────────────────────────────────────────────────┤
│                  Comparison (derived)                        │
│  Comparison, ScorerComparison, SampleComparison, Change      │
├─────────────────────────────────────────────────────────────┤
│                  Serialization (convenience)                 │
│  JSONL reader/writer                                         │
└─────────────────────────────────────────────────────────────┘

Feature-gated:
  [otel]       → Observe acquisition, TraceBackend, JaegerBackend
  [llm-judge]  → LLM-as-a-Judge scorer
```

### 4.2 Data Flow

**Inline evaluation (most common):**

```
acquisition.acquire(input) → output O
  → global map_output (O → O2, if configured)
  → global map_reference (R → R2, if configured)
  → for each scorer / scorer_set:
      → per-set map_output (O2 → O3, if configured)
      → per-set map_reference (R2 → R3, if configured)
      → scorer.score(ScorerContext { input, output, reference })
      → Result<Score, ScorerError>
  → collect TrialResult
→ repeat for N trials
→ collect SampleResult
→ repeat for all samples (sequential)
→ RunResult
```

**OTel observe mode:**

```
(agent runs externally, emits OTel spans with baggage attributes)
  → observe.acquire(input):
      → backend.fetch_spans(correlation_id, sample_attribute, timeout)
      → returns Vec<Span> for this sample
  → global map_output (Vec<Span> → domain type, typically required)
  → scoring proceeds identically to inline mode
```

### 4.3 Architectural Decisions

| ID | Decision | Options Considered | Choice | Rationale | Research Reference |
|----|----------|-------------------|--------|-----------|-------------------|
| AD-01 | Generic scorer trait with AI-specific convenience layers | (1) Fully generic `Scorer<I, O, R>` (2) AI-specific core types (3) Type-erased `serde_json::Value` | Generic trait with AI aliases | Generic enables non-AI use and composability; aliases prevent verbosity for the common case. Every existing tool bakes AI into the core — this is the novel approach. | 01-landscape.md: no generic-core eval framework exists; braindump H-01 |
| AD-02 | Scorer returns Result, not bare Score | (1) `Result<Score, ScorerError>` (2) `Score` with error variant (3) `Score` only | Result<Score, ScorerError> | Conflating errors with low scores hides infrastructure problems. Every major framework distinguishes these (DeepEval: metric.error, Inspect AI: EvalError, Promptfoo: ResultFailureReason). | 04-architecture.md Decision 1; subagent research Round 6 |
| AD-03 | Score as enum, not trait | (1) Enum (Numeric, Binary, Label, Metric) (2) Trait with impls (3) Single f64 | Enum | Finite set enables exhaustive matching, serialization, and type-appropriate aggregation. Stats dispatch on variant. | brainstorm Direction 1 core technical decisions |
| AD-04 | Acquisition as trait with Fn blanket impl | (1) Trait with Inline wrapper (2) Trait with Fn blanket impl (3) Enum | Trait with blanket impl | Closures just work as acquisitions. Custom impls use the trait. No wrapper needed for the common case. | braindump I-02: multiple execution modes; user suggestion Round 3 |
| AD-05 | Observe mode: framework doesn't call agent | (1) Framework sends HTTP request with traceparent (2) Framework only queries traces (3) Both | Framework only queries traces | Massive simplification. Truly agent-agnostic. Historical and live traces are the same operation. | User insight during Round 1; agentevals-dev pattern (01-cross-reference.md) |
| AD-06 | Unified Mapper trait for output and reference transforms | (1) Separate Transform and ReferenceMap traits (2) Unified Mapper trait (3) Closures only | Unified Mapper | Structurally identical operations. One trait, different builder methods (`.map_output()`, `.map_reference()`). | User observation Round 2 |
| AD-07 | ScorerContext struct instead of function parameters | (1) Individual params `(input, output, reference)` (2) `ScorerContext` struct | ScorerContext with `#[non_exhaustive]` | Adding fields later (metadata, trial info, cost) is non-breaking. Protects scorer implementations from API changes. | User suggestion Round 2 |
| AD-08 | ScoreDefinition as first-class type | (1) Separate `name()` and `direction()` methods (2) `ScoreDefinition` struct | ScoreDefinition | Formalizes "what this score means" separate from the scorer instance and the score value. Extensible (can add description, unit, value range). Persisted in RunMetadata. | User insight Round 3 (measurement analogy) |
| AD-09 | Direction as `Option<Direction>` (Maximize/Minimize) | (1) `Direction` with Neutral variant (2) `Option<Direction>` (3) Always required | `Option<Direction>` | Binary and Label scores have no ordering — direction doesn't apply. `None` = concept is irrelevant, not "neutral." | User discussion Round 3 |
| AD-10 | Stats computed separately from RunResult | (1) Stats baked into RunResult (2) Stats computed via `.stats()` (3) Separate aggregation step | Separate computation | Decouples "what happened" from "what it means." Enables custom aggregation. RunResult is raw data. | User insight Round 3 |
| AD-11 | Sequential execution for MVP | (1) Concurrent from day one (2) Sequential MVP, concurrent later | Sequential MVP | Simpler, debuggable, predictable. `.concurrency(N)` API designed but defaults to 1. Concurrent executor is a non-breaking addition. | User suggestion Round 4 |
| AD-12 | Single crate, flat structure, no mod.rs | (1) Workspace with multiple crates (2) Single crate with `core/` submodule (3) Flat crate structure | Flat single crate | Idiomatic Rust for small-medium crates. Feature gates for OTel and LLM-as-a-Judge. No unnecessary nesting. | User guidance Round 4 |
| AD-13 | Persistence decoupled from core, convenience provided | (1) Built-in file I/O (2) Serde only, no convenience (3) Serde + JSONL convenience | Serde + JSONL convenience | Core API unaffected by persistence. JSONL convenience has no deps beyond serde. Like stats — a good default near the core, not coupled to it. | User clarification Round 3; 06-ecosystem.md: JSONL is de facto standard |

---

## 5. API Surface

### 5.0 Prelude

```rust
pub mod prelude {
    pub use crate::{
        Sample, Dataset, Score, ScoreDefinition, Direction,
        Scorer, ScorerContext, ScorerError, ScorerSet,
        Mapper, Acquisition, Run, RunResult,
        scorers::*,
    };
}
```

### 5.1 Core Types

#### `Sample<I, R>`

```rust
pub struct Sample<I, R = ()> {
    pub id: String,
    pub input: I,
    pub reference: Option<R>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<I: Hash, R: Hash> Sample<I, R> {
    /// Creates a sample with auto-generated deterministic ID (content-hashed).
    pub fn new(input: I, reference: R) -> Self { ... }
}

impl<I, R> Sample<I, R> {
    /// Builder for samples with explicit ID or no reference.
    pub fn builder(input: I) -> SampleBuilder<I, R> { ... }
}

// SampleBuilder methods:
//   .id(id: impl Into<String>) -> Self
//   .reference(reference: R) -> Self
//   .metadata(key: impl Into<String>, value: serde_json::Value) -> Self
//   .build() -> Sample<I, R>
```

#### `Dataset<I, R>`

```rust
pub struct Dataset<I, R = ()> {
    pub samples: Vec<Sample<I, R>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<I, R> Dataset<I, R> {
    pub fn new(samples: Vec<Sample<I, R>>) -> Self { ... }
}

// Also: impl<I, R> From<Vec<Sample<I, R>>> for Dataset<I, R>
```

#### `Score`

```rust
#[non_exhaustive]
pub enum Score {
    Numeric(f64),
    Binary(bool),
    Label(String),
    Metric { name: String, value: f64, unit: Option<String> },
}
```

#### `ScoreDefinition`

```rust
pub struct ScoreDefinition {
    pub name: String,
    pub direction: Option<Direction>,
}

pub enum Direction {
    Maximize,
    Minimize,
}

impl ScoreDefinition {
    pub fn new(name: impl Into<String>) -> Self { ... }           // direction: None
    pub fn maximize(name: impl Into<String>) -> Self { ... }      // direction: Some(Maximize)
    pub fn minimize(name: impl Into<String>) -> Self { ... }      // direction: Some(Minimize)
}
```

#### `ScorerError`

```rust
pub struct ScorerError(pub Box<dyn std::error::Error + Send + Sync>);
```

### 5.2 Scorer Trait

```rust
#[non_exhaustive]
pub struct ScorerContext<'a, I, O, R = ()> {
    pub input: &'a I,
    pub output: &'a O,
    pub reference: Option<&'a R>,
}

pub trait Scorer<I, O, R = ()>: Send + Sync {
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError>;
    fn definition(&self) -> ScoreDefinition;
}
```

### 5.3 Built-in Scorers

```rust
pub fn exact_match() -> impl Scorer<String, String, String>
pub fn contains() -> impl Scorer<String, String, String>
pub fn regex(pattern: &str) -> Result<impl Scorer<String, String>, regex::Error>  // R defaults to () — no reference needed
pub fn json_schema(schema: serde_json::Value) -> impl Scorer<String, String>

// [llm-judge] feature
pub fn llm_judge(config: LlmJudgeConfig) -> impl Scorer<String, String, String>
```

### 5.4 Error Types

```rust
/// Error from a Mapper (output or reference mapping)
pub struct MapError(pub Box<dyn std::error::Error + Send + Sync>);

/// Error from an Acquisition
pub enum AcquisitionError {
    /// The acquisition function returned an error or panicked
    ExecutionFailed(Box<dyn std::error::Error + Send + Sync>),
    /// No spans matched the correlation + sample attribute [otel]
    TraceNotFound { correlation_id: String, sample_id: String },
    /// Trace backend is unreachable [otel]
    BackendUnavailable(Box<dyn std::error::Error + Send + Sync>),
    /// Acquisition exceeded sample_timeout
    Timeout(Duration),
}

/// Error from a TraceBackend [otel]
pub struct TraceBackendError(pub Box<dyn std::error::Error + Send + Sync>);

/// Error building a Run
#[non_exhaustive]
pub enum RunBuildError {
    NoDataset,
    NoAcquisition,
    NoScorer,
    EmptyDataset,
    DuplicateSampleIds(Vec<String>),
    DuplicateScorerNames(String),
    MissingSampleIds,
}

/// Error executing a Run
#[non_exhaustive]
pub enum RunError {
    Build(RunBuildError),
    Internal(Box<dyn std::error::Error + Send + Sync>),
}
```

All error types implement `std::fmt::Display`, `std::fmt::Debug`, and `std::error::Error`.

### 5.5 Mapper Trait

```rust
pub trait Mapper<I, O>: Send + Sync {
    fn map(&self, input: &I) -> Result<O, MapError>;
}

// Blanket impl for closures
impl<F, I, O> Mapper<I, O> for F
where F: Fn(&I) -> Result<O, MapError> + Send + Sync { ... }
```

### 5.5 ScorerSet

```rust
ScorerSet::builder()
    .map_output(output_mapper)       // optional: Mapper<O, O2>
    .map_reference(reference_mapper) // optional: Mapper<R, R2>
    .scorer(scorer_a)
    .scorer(scorer_b)
    .build()
```

Mapper runs once per trial; result shared by all scorers in the set.

### 5.6 Acquisition Trait

```rust
pub trait Acquisition<I, O>: Send + Sync {
    async fn acquire(&self, input: &I) -> Result<O, AcquisitionError>;
}

// Blanket impl: closures are Acquisitions
impl<I, O, F, Fut> Acquisition<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: Future<Output = Result<O, AcquisitionError>> + Send,
{ ... }
```

**Observe mode** (`otel` feature):

```rust
let observe = Observe::builder()
    .backend(JaegerBackend::new("http://localhost:16686"))
    .correlation_id("run-abc-123")
    .sample_attribute("eval.sample_id")
    .timeout(Duration::from_secs(30))
    .build();

// Observe implements Acquisition<I, Vec<Span>>
// Extraction from spans is done via Mapper (map_output on Run or ScorerSet)
```

#### `Span` type (`otel` feature)

```rust
/// Represents an OTel span. Minimal type — users access attributes by name.
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
}

pub struct SpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
}
```

### 5.7 TraceBackend Trait

```rust
pub trait TraceBackend: Send + Sync {
    async fn fetch_spans(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError>;
}
```

Built-in: `JaegerBackend` (Jaeger v2 API, `otel` feature).

### 5.8 Run Builder

```rust
Run::builder()
    .dataset(samples)                           // required
    .acquisition(|input: &String| async { .. }) // required (closure or impl Acquisition)
    .map_output(global_mapper)                  // optional global output mapper
    .map_reference(global_ref_mapper)           // optional global reference mapper
    .scorer(exact_match())                      // at least one scorer or scorer_set
    .scorer_set(scorer_set)                     // optional
    .trials(10)                                 // optional, default 1
    .concurrency(4)                             // optional, default 1 (sequential). Reserved for future concurrent execution.
    .sample_timeout(Duration::from_secs(30))    // optional
    .build()?                                   // returns Result<Run, RunBuildError>
    .execute()                                  // returns Result<RunResult, RunError>
    .await?;
```

**Builder ordering:** `.dataset()` and `.acquisition()` first, then `.map_output()` / `.map_reference()` (if any), then `.scorer()` / `.scorer_set()`, then options. Enforced by type system — mappers change the output/reference types that subsequent scorers must match.

### 5.9 Stats API

```rust
impl RunResult {
    /// Compute statistics from raw results (default confidence level 0.95)
    pub fn stats(&self) -> RunStats { ... }

    /// Compute statistics with custom confidence level
    pub fn stats_with(&self, confidence_level: f64) -> RunStats { ... }
}

pub struct RunStats {
    /// Per-scorer stats aggregated across all samples
    pub scorer_stats: HashMap<String, ScorerStats>,
    pub total_samples: usize,
    pub total_trials: usize,
    pub total_errors: usize,
}

impl RunStats {
    /// Human-readable summary string
    pub fn summary(&self) -> String { ... }
}
```

### 5.10 Comparison API

```rust
pub fn compare(
    baseline: &RunResult,
    candidate: &RunResult,
    config: CompareConfig,
) -> Comparison { ... }

pub struct CompareConfig {
    pub confidence_level: f64,  // default 0.95
}

impl Default for CompareConfig {
    fn default() -> Self { Self { confidence_level: 0.95 } }
}
```

### 5.11 JSONL Convenience

```rust
/// Write a RunResult as JSONL
pub fn write_jsonl(result: &RunResult, writer: impl Write) -> Result<(), serde_json::Error>

/// Read a RunResult from JSONL
pub fn read_jsonl(reader: impl Read) -> Result<RunResult, serde_json::Error>
```

### 5.12 Hello World

```rust
use evalkit::prelude::*;  // crate name TBD — see OQ-01

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples = vec![
        Sample::new("What is 2+2?", "4"),
        Sample::new("Capital of France?", "Paris"),
    ];

    let result = Run::builder()
        .dataset(samples)
        .acquisition(|input: &String| async {
            Ok(my_agent(input).await)
        })
        .scorer(exact_match())
        .build()?
        .execute()
        .await?;

    let stats = result.stats();
    println!("{}", stats.summary());
    // Output:
    // Run complete: 2 samples, 1 scorer, 1 trial
    // exact_match: mean=0.50, pass_rate=50.0% (1/2)

    Ok(())
}
```

---

## 6. Data Model

### 6.1 Schema

#### Sample<I, R>

| Field | Type | Required | Default | Constraints | Description |
|-------|------|----------|---------|-------------|-------------|
| `id` | `String` | Yes | Content-hashed (input + reference) | Non-empty, unique within dataset | Stable identity for cross-run comparison and observe-mode matching |
| `input` | `I` | Yes | — | `Send + Sync + Clone` | Input to the system under evaluation |
| `reference` | `Option<R>` | No | `None` | `Send + Sync + Clone` when present | Reference value for comparison |
| `metadata` | `HashMap<String, Value>` | No | Empty | — | Arbitrary tags and annotations |

**Serialization:** JSON. `I` and `R` must implement `Serialize + DeserializeOwned`.
**Lifecycle:** Created by user → immutable during Run → referenced in results.

#### Score

| Variant | Fields | Constraints | Description |
|---------|--------|-------------|-------------|
| `Numeric` | `value: f64` | Finite (not NaN, not Infinity) | Continuous score |
| `Binary` | `value: bool` | — | Pass/fail |
| `Label` | `value: String` | Non-empty | Categorical classification |
| `Metric` | `name: String, value: f64, unit: Option<String>` | name non-empty, value finite | Named measurement |

**Validation:** Framework validates after every scorer call. Invalid scores become `ScorerError`.

**Serialization:** Tagged JSON: `{"type": "numeric", "value": 0.85}`.

#### TrialResult

| Field | Type | Required | Constraints | Description |
|-------|------|----------|-------------|-------------|
| `scores` | `HashMap<String, Result<Score, ScorerError>>` | Yes | Keys are scorer names, at least one entry | Score or error per scorer |
| `duration` | `Duration` | Yes | Non-negative | Wall-clock time for this trial |
| `trial_index` | `usize` | Yes | 0-based | Which trial (0..N) |

#### SampleResult

| Field | Type | Required | Constraints | Description |
|-------|------|----------|-------------|-------------|
| `sample_id` | `String` | Yes | Matches a Sample.id | Which sample |
| `trials` | `Vec<TrialResult>` | Yes | Length = trial count | Raw results per trial |
| `trial_count` | `usize` | Yes | > 0 | Total trials attempted |
| `scored_count` | `usize` | Yes | <= trial_count | Trials that produced at least one score |
| `error_count` | `usize` | Yes | = trial_count - scored_count | Trials where ALL scorers errored. A trial with partial scorer success counts as scored. |

#### RunMetadata

| Field | Type | Required | Constraints | Description |
|-------|------|----------|-------------|-------------|
| `run_id` | `String` | Yes | Non-empty, auto-generated UUID | Unique run identifier |
| `started_at` | `DateTime<Utc>` | Yes | — | Execution start time |
| `completed_at` | `DateTime<Utc>` | Yes | >= started_at | Execution end time |
| `duration` | `Duration` | Yes | — | Total wall-clock time |
| `trial_count` | `usize` | Yes | > 0 | Trials per sample |
| `score_definitions` | `Vec<ScoreDefinition>` | Yes | Non-empty | Definitions for all scorers used |
| `acquisition_mode` | `String` | Yes | "inline" or "observe" | Which acquisition mode |

#### RunResult

| Field | Type | Required | Constraints | Description |
|-------|------|----------|-------------|-------------|
| `metadata` | `RunMetadata` | Yes | — | Run execution metadata |
| `samples` | `Vec<SampleResult>` | Yes | Same length and order as input dataset | Per-sample results |

**Lifecycle:** Created by `Run::execute()` → immutable → caller serializes/analyzes as needed.

#### ScorerStats (computed, not stored in RunResult)

| Variant | Fields | Description |
|---------|--------|-------------|
| `Numeric` | `mean, stddev, ci: (f64, f64), min, max` | t-distribution CI |
| `Binary` | `pass_rate, pass_at_k, pass_all_k, ci: (f64, f64)` | Wilson CI |
| `Label` | `distribution: HashMap<String, usize>, mode: String` | Frequency distribution |
| `Metric` | `mean, stddev, ci: (f64, f64), min, max` | Same shape as Numeric |

Computed from scored trials only. Errored trials excluded from denominator.

#### Comparison (computed)

| Field | Type | Description |
|-------|------|-------------|
| `baseline_id` | `String` | Run ID of baseline |
| `candidate_id` | `String` | Run ID of candidate |
| `shared_scorers` | `HashMap<String, ScorerComparison>` | Scorers in both runs |
| `only_in_baseline` | `Vec<String>` | Scorers only in baseline |
| `only_in_candidate` | `Vec<String>` | Scorers only in candidate |
| `confidence_level` | `f64` | Confidence level for significance tests |

#### ScorerComparison

| Field | Type | Description |
|-------|------|-------------|
| `sample_comparisons` | `HashMap<String, SampleComparison>` | Per-sample deltas |
| `aggregate_delta` | `f64` | Difference in aggregate (candidate - baseline). For Numeric/Metric: difference in mean. For Binary: difference in pass rate. For Label: 0.0 (use `Change` variant instead). |
| `p_value` | `Option<f64>` | Significance test p-value |
| `significant` | `Option<bool>` | Significant at confidence level? |
| `test_used` | `Option<String>` | Statistical test name |

#### SampleComparison

| Field | Type | Description |
|-------|------|-------------|
| `sample_id` | `String` | Which sample |
| `delta` | `f64` | Score difference (candidate - baseline) |
| `direction` | `Change` | Improved / Regressed / Unchanged / Insignificant / Incomparable |

#### Change (enum)

| Variant | Meaning |
|---------|---------|
| `Improved` | Candidate is better than baseline (direction-aware) |
| `Regressed` | Candidate is worse than baseline (direction-aware) |
| `Unchanged` | No meaningful difference |
| `Insignificant` | Delta exists but not statistically significant |
| `Incomparable` | Different score types, direction mismatch, or insufficient data |

### 6.2 Persistence Strategy

All public types implement `serde::Serialize + serde::Deserialize`. The library provides JSONL convenience functions for `RunResult` (zero dependencies beyond serde). Persistence decisions (file, database, stdout) are the caller's responsibility.

**Why JSONL:** Line-oriented (streamable), human-readable, git-friendly diffs, de facto standard in the eval domain (06-ecosystem.md). Recommended format, not enforced.

**Deterministic serialization:** Same `RunResult` produces the same bytes (sorted keys, consistent formatting) to enable meaningful `git diff`.

---

## 7. Integration Points

### Integration: Jaeger v2 API
- **Direction**: Outbound (`otel` feature)
- **Protocol**: HTTP/JSON
- **Authentication**: None by default; configurable custom headers
- **Data exchanged**: Query by trace attributes → spans as JSON
- **Failure mode**: Timeout/unreachable → `AcquisitionError` per sample. Configurable timeout and retry count on backend.
- **Version coupling**: Low. Jaeger API is stable. Only `JaegerBackend` impl affected by API changes.

### Integration: LLM Provider APIs
- **Direction**: Outbound (`llm-judge` feature)
- **Protocol**: HTTP/JSON (OpenAI-compatible)
- **Authentication**: API key via config
- **Data exchanged**: Chat completion request → response parsed into Score
- **Failure mode**: Network/parse errors → `ScorerError`, distinct from low scores
- **Version coupling**: Low. The LLM-as-a-Judge scorer is a thin impl. Users can write their own for any provider.

### Integration: OTel-instrumented agents
- **Direction**: Indirect — framework doesn't talk to agents
- **Protocol**: Convention-based OTel baggage attributes (`eval.run_id`, `eval.sample_id`)
- **Failure mode**: Missing/incorrect attributes → `AcquisitionError` for affected samples
- **Version coupling**: Built-in mappers target stable subset of OTel GenAI semantic conventions. User-extensible via custom Mapper impls.

### Integration: serde ecosystem
- **Direction**: Internal dependency
- **Failure mode**: Serialization errors propagated to caller
- **Version coupling**: Effectively zero (serde is the most stable Rust crate)

### Integration: tokio async runtime
- **Direction**: Internal dependency
- **Version coupling**: Low (tokio API is stable). Runtime-agnostic abstraction not worth the complexity for MVP.

---

## 8. Error Handling

### 8.1 Error Taxonomy

#### RunBuildError — configuration problems (pre-execution)

| Error | When it occurs | User sees | Recovery |
|-------|---------------|-----------|----------|
| `NoDataset` | `.build()` without `.dataset()` | `Err(RunBuildError)` | Add `.dataset()` |
| `NoAcquisition` | `.build()` without `.acquisition()` | `Err(RunBuildError)` | Add `.acquisition()` |
| `NoScorer` | `.build()` without any scorer | `Err(RunBuildError)` | Add `.scorer()` or `.scorer_set()` |
| `EmptyDataset` | Dataset has zero samples | `Err(RunBuildError)` | Add samples |
| `DuplicateSampleIds` | Two samples share an ID | `Err(RunBuildError)` with duplicate IDs | Ensure unique IDs |
| `DuplicateScorerNames` | Two scorers share a definition name | `Err(RunBuildError)` with duplicate name | Rename one scorer |
| `MissingSampleIds` | Observe mode with auto-generated IDs | `Err(RunBuildError)` | Provide explicit IDs |

#### RunError — framework-level failures

| Error | When it occurs | User sees | Recovery |
|-------|---------------|-----------|----------|
| `Build(RunBuildError)` | Build validation failed | `Err(RunError::Build(...))` | Fix configuration |
| `Internal(...)` | Framework bug | `Err(RunError::Internal(...))` | File a bug report |

**Note:** `execute()` returns `Ok(RunResult)` for all evaluation outcomes, including partial/total sample failures. `Err` only for things that prevent producing ANY result.

#### AcquisitionError — per-sample output acquisition failures

| Error | When it occurs | Effect | Recovery |
|-------|---------------|--------|----------|
| `ExecutionFailed(...)` | Acquisition function returned error or panicked | SampleResult with error, other samples continue | Fix agent/function |
| `TraceNotFound` | No spans matched correlation + sample attribute (`otel`) | SampleResult with error | Check OTel attributes, increase timeout |
| `BackendUnavailable` | Trace backend unreachable (`otel`) | SampleResult with error | Check backend connectivity |
| `Timeout` | Acquisition exceeded sample_timeout | SampleResult with error | Increase timeout or fix agent |

#### ScorerError — per-scorer failures

| Cause | Effect | Recovery |
|-------|--------|---------|
| Network failure (LLM-as-a-Judge) | Error for this scorer in TrialResult. Other scorers continue. | Check network/API key |
| Parse failure | Same | Check output format |
| Mapper failure | Error for ALL scorers in affected ScorerSet | Fix mapper logic |
| Score validation failure (NaN, empty label) | Converted to ScorerError | Fix scorer logic |

### 8.2 Error Propagation Rules

| Error type | Scope | Other operations | RunResult returned? |
|-----------|-------|-----------------|---------------------|
| RunBuildError | Pre-execution | Nothing starts | No |
| RunError::Internal | Entire run | Everything stops | No |
| AcquisitionError | Per sample, per trial | Other samples/trials continue | Yes |
| ScorerError | Per scorer, per trial | Other scorers/trials continue | Yes |
| MapError | Per ScorerSet, per trial | Scorers outside set continue | Yes |

### 8.3 Interrupted Operation Recovery

**SIGKILL / power failure:** No cleanup needed. The library produces no side effects — no files written, no external state mutated. A killed run simply produces no result.

**SIGINT (Ctrl+C):** tokio cancellation propagates. In-flight operations cancelled. No partial RunResult in MVP.

**Panics in user code:** Caught via `tokio::spawn` + `JoinHandle`. Converted to `AcquisitionError` or `ScorerError`. Other samples/trials continue. The run does not crash.

---

## 9. Constraints & Quality Attributes

### Performance
- **Scorer overhead**: < 1ms per scorer call (framework overhead, excluding scorer computation) — measured by no-op scorer benchmark across 1000 samples
- **Result serialization**: < 100ms for 1000-sample, 10-scorer RunResult to JSONL
- **Result deserialization**: < 100ms for same

### Correctness
- **Statistical functions**: Wilson CI, t-distribution CI, significance tests match reference implementations (R, SciPy) to within 1e-6
- **Score validation**: Every Score validated post-scorer (finite numerics, non-empty labels)
- **Deterministic auto-IDs**: Same Sample content → same ID across runs, platforms, Rust versions

### Security
- **API keys**: Never appear in RunResult, serialized output, error messages, or panic messages
- **No network without feature gates**: Default features make zero network connections
- **No arbitrary code execution**: Scorers are compiled Rust. Regex crate prevents ReDoS.

### Compatibility
- **Rust edition**: 2024 (or latest stable)
- **MSRV**: Latest stable at first release
- **Platforms**: Linux x86_64 (primary), macOS arm64 (supported), Windows x86_64 (best-effort)
- **Async runtime**: tokio

### Resource Limits
- **Memory**: < 10MB framework overhead for typical workloads (< 1000 samples, < 10 scorers)
- **Dependencies (default)**: serde, serde_json, tokio, regex. < 30 total crates.
- **Dependencies (otel)**: Adds HTTP client. < 20 additional crates.

### API Stability
- **Pre-1.0**: No stability guarantees
- **`#[non_exhaustive]`** on: Score, ScorerContext, RunError, RunBuildError
- **Semver**: Rust conventions

---

## 10. Security Considerations

### Threat Model

**In scope:**
- API key leakage via serialization or error messages
- ReDoS via regex scorer
- Untrusted scorer output (NaN, malformed data)

**Out of scope (not applicable to a library):**
- Authentication/authorization (no network service)
- Data at rest encryption (caller's responsibility)
- Multi-tenant isolation (single-user library)

**Trust boundaries:**
- User code (scorers, acquisition functions, mappers) is trusted — it runs in-process
- External APIs (LLM providers, trace backends) are untrusted — responses are validated
- OTel span data is untrusted — extraction via Mapper, user validates

**Mitigations:**
- `#[serde(skip)]` on sensitive fields (API keys in LlmJudgeConfig)
- Score validation post-scorer catches NaN/infinity injection
- `regex` crate guarantees linear-time matching (no ReDoS)
- No `unsafe` code in the library

---

## 11. Dependency Graph

### 11.1 Internal Components

```
Sample, Dataset (no dependencies)
Score, ScoreDefinition, Direction (no dependencies)
ScorerContext (no dependencies — generic over I, O, R)
ScorerError (no dependencies)
Mapper (no dependencies)
Scorer trait (depends on Score, ScorerContext, ScorerError, ScoreDefinition)
ScorerSet (depends on Scorer, Mapper)
Acquisition trait (no dependencies)
Built-in scorers (depends on Scorer trait)
Run builder (depends on Sample, Scorer, ScorerSet, Mapper, Acquisition)
RunResult, SampleResult, TrialResult (depends on Score, ScorerError, ScoreDefinition)
Stats (depends on RunResult, Score)
Comparison (depends on RunResult, Stats, ScoreDefinition, Direction)
JSONL (depends on RunResult, serde)
Observe acquisition (depends on Acquisition, TraceBackend) [otel]
TraceBackend, JaegerBackend (no internal deps) [otel]
LLM-as-a-Judge scorer (depends on Scorer trait) [llm-judge]
```

### 11.2 External Dependencies

| Dependency | Version/Spec | Why | Risk if Unavailable |
|-----------|-------------|-----|---------------------|
| serde + serde_json | 1.x | Serialization of all public types | Cannot serialize results. Core functionality loss. |
| tokio | 1.x | Async runtime for scorer/acquisition execution | Cannot run async scorers. Fundamental dependency. |
| regex | 1.x | Built-in regex scorer. Linear-time guarantee. | Regex scorer unavailable. Minor. |
| chrono | 0.4.x | DateTime in RunMetadata | Could use std::time instead. Low risk. |
| uuid | 1.x | Auto-generated run IDs | Could use random bytes. Low risk. |
| reqwest (or ureq) | Latest | HTTP client for Jaeger API and LLM-as-a-Judge | `otel` and `llm-judge` features unavailable. Core unaffected. |

---

## 12. Validation Checklist

### Failure Archaeology Cross-Check

| Failed Project | Failure Cause | Our Approach | Different Because |
|---------------|--------------|-------------|-------------------|
| Log10 (archived) | Minimal differentiation in crowded observability space | Four differentiation axes: Rust, generic core, OTel observe, statistical rigor | Not in observability space. Substantial differentiation. |
| AIConfig (stale) | Standalone prompt config absorbed into platforms | Not a product targeting a market. Building for one user. | No market dependency. |
| HuggingFace Evaluate (deprecated) | Traditional NLP metrics irrelevant for LLMs | Generic scorer trait, user-extensible | Framework doesn't become obsolete when methodology changes |
| Metrics-only library pattern | Teams need platform features beyond just metrics | **Acknowledged risk.** Mitigated by: building for one user, not a market. | If external adoption matters later, platform is a separate project. |
| Single-provider coupling | Tools tied to one provider lose users | Provider-agnostic by design. Zero provider deps in core. | Structural guarantee. |

### Growth Path Compatibility

| Future Capability | Breaking Change Required? | Mitigation |
|------------------|--------------------------|------------|
| Scorer composition (.and, .weighted, .then) | No | Extension methods on Scorer trait |
| Concurrent execution | No | Implementation detail behind existing `.concurrency(N)` API |
| Python bindings (PyO3) | No | No complex lifetimes in public API. `#[non_exhaustive]` on key types. |
| OTLP receiver | No | New TraceBackend implementation |
| Additional trace backends (Tempo) | No | New TraceBackend implementation |
| Drift detection (CUSUM) | No | New functions in stats module on `Vec<RunResult>` |
| Adaptive trial counts | No | New builder option |
| Cost tracking | No | New fields in `ScorerContext` (`#[non_exhaustive]`) |
| FailureThreshold / abort on error rate | No | New builder option + new RunError variant |
| Score::Labels (multi-label) | No | New enum variant (`#[non_exhaustive]`) |
| CLI tool | No | Separate crate consuming this library |

---

## 13. Open Questions

- **[OQ-01]**: Crate naming — generic (evalkit, scored) vs. domain-leaning (agenteval)?
  - Impact: Package identity, discoverability on crates.io, import paths
  - Deadline: Before first publish to crates.io

- **[OQ-02]**: How should scorer metadata (timing, cost) be attached to results?
  - Impact: TrialResult structure, ScorerContext fields
  - Deadline: During implementation — can defer to post-MVP via `#[non_exhaustive]`

- **[OQ-03]**: Should the framework define its own OTel semantic conventions for evaluation results (e.g., `eval.run.id`, `eval.sample.id` span attributes)?
  - Impact: OTel integration documentation, interoperability with other tools
  - Deadline: Before OTel feature stabilizes

- **[OQ-04]**: What's the right default trial count?
  - Impact: Default behavior of Run builder
  - Deadline: During implementation — start with 1, adjust based on user experience

- **[OQ-05]**: How should results from different trial counts be compared? (Run A: 10 trials, Run B: 20 trials)
  - Impact: Comparison logic in compare module
  - Deadline: During implementation of comparison feature

- **[OQ-06]**: Should the Scorer trait design be validated via the proposed design spike (3 variations against 4 real workflows) before committing?
  - Impact: Potentially changes the core trait design
  - Deadline: Before implementation begins

---

## 14. Glossary

### Core Concepts

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `evaluation` / `eval` | The process of measuring an AI system's quality by giving it inputs and scoring its outputs. | Aligned — "eval" is universal shorthand (Anthropic, DeepEval, Inspect AI, all major tools). | |
| `Sample` | A single evaluation data point: an input to the system under evaluation, plus an optional reference value for comparison. Identified by a stable `id` (content-hashed by default). | Fractured — "Task" (Inspect AI, Anthropic), "Test Case" (DeepEval), "Sample" (RAGAS), "Problem", "Scenario". | Deliberate choice: neutral, non-test-framework, avoids Rust keyword conflicts (`test`, `case`). Follows RAGAS convention. |
| `Dataset` | An ordered collection of Samples. | Aligned — "Dataset" is universal across the domain. | Some tools distinguish "Dataset" (data only) from "Test Suite" (data + assertions). This project uses Dataset for both since assertions live in Scorers, not in data. |
| `reference` | The expected or comparison value in a Sample. Used by scorers that compare the system's output against a known-good value. `Option<R>` — not all evaluations need one. | Fractured — "Ground Truth" (Microsoft, AWS), "Golden Answer", "Reference" (RAGAS), "Target" (Inspect AI), "Expected Output" (DeepEval), "Expected" (Braintrust, Promptfoo). | Deliberate choice: "reference" is neutral about correctness — many evaluations have no single correct answer, only a reference point. Follows RAGAS convention. |

### Scoring

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

### Mapping & Transformation

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Mapper` | A type conversion trait used for both output mapping (`O → O2`) and reference mapping (`R → R2`). Closures implement it automatically via blanket impl. | No domain equivalent in eval tools. DeepEval has implicit preprocessing. | Novel in the eval domain. Familiar from `Iterator::map` in Rust and functional programming. Unifies what would be separate "transform" and "reference converter" concepts under one trait. |
| `MapError` | Error from a Mapper (output or reference mapping). When a mapper in a ScorerSet fails, all scorers in that set receive this as a ScorerError. | No standard. | Follows from Mapper. |

### Acquisition

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Acquisition` | How outputs are obtained for evaluation. The architectural boundary between "get the output" and "score it." A trait with two built-in implementations: inline (closure) and observe (OTel traces). | No domain equivalent — every other eval tool either assumes inline execution or (agentevals-dev) evaluates from traces without formalizing the abstraction. | Novel. Names a previously-unnamed concept. In inline mode, acquisition calls a function and receives its return value. In observe mode, acquisition queries a trace backend and returns spans. |
| `inline` | Acquisition mode: the framework calls a user-provided function (closure) to get the output. The default mode and the way most eval tools work. | No standard — this is the implicit default in all eval tools, but no tool names it. | Novel. |
| `observe` | Acquisition mode: the framework queries existing OTel traces to extract outputs. The agent runs externally; the framework only reads spans. Enables evaluation without re-execution. | No standard. agentevals-dev does this but doesn't name the mode. | Novel. The framework does NOT call the agent — it only reads traces. Historical and live traces are the same operation. |
| `correlation_id` | A domain-level identifier (e.g., a run ID or execution ID) used by observe-mode to query a TraceBackend for matching spans. The framework groups returned spans per sample using the `sample_attribute`. | Novel — no standard equivalent. OTel has "trace context" for distributed tracing propagation, but correlation here is done via OTel baggage/attributes, not trace IDs. | An agent execution may span multiple OTel traces. Correlation by domain-level ID (via baggage attributes like `eval.run_id`) is more robust than correlation by trace ID. |
| `sample_attribute` | The name of the OTel span attribute used to group spans per sample in observe mode. E.g., `"eval.sample_id"`. | No standard. | Configurable per Observe builder. The TraceBackend groups spans by this attribute's value, matching them to Sample IDs. |
| `AcquisitionError` | Error from obtaining output for a sample. Variants: `ExecutionFailed` (function error/panic), `TraceNotFound` (no matching spans), `BackendUnavailable` (trace backend unreachable), `Timeout` (exceeded sample_timeout). | No standard. | Per-sample, per-trial error. Other samples/trials continue. |

### OTel / Observability

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Trace` | An OpenTelemetry execution record — a tree of spans representing one request or operation through a system. | Aligned — "Trace" is the OTel standard term, universal in observability. | In evaluation context, traces contain the evidence of what an agent did. Distinct from Trajectory (see below). |
| `Span` | An individual operation within a trace: has a trace ID, span ID, operation name, timestamps, attributes, and events. | Aligned — "Span" is the OTel standard term. | Represented as a struct with `attributes: HashMap<String, Value>` for flexible access. |
| `SpanEvent` | An event within a Span: a named, timestamped occurrence with attributes. | Aligned — OTel standard concept (Span Events). | |
| `Trajectory` | The evaluation-relevant sequence of agent decisions and actions. Extracted from a Trace for evaluation purposes. | Aligned — "Trajectory" (LangChain AgentEvals, Anthropic, Arize). | Distinct from Trace: a trace is an observability record; a trajectory is the subset relevant to evaluating agent behavior. In this framework, trajectory extraction is done via a Mapper on `Vec<Span>`. |
| `TraceBackend` | A pluggable source for querying OTel spans. Given a correlation ID and sample attribute, returns matching spans grouped by sample. | No standard. | Novel. Built-in: `JaegerBackend` (Jaeger v2 API). Future: Tempo, OTLP receiver. Users can implement custom backends. |
| `TraceBackendError` | Error from a TraceBackend query (e.g., backend unreachable, query timeout, malformed response). | No standard. | Follows from TraceBackend. Wrapped into `AcquisitionError::BackendUnavailable` at the acquisition layer. |

### Evaluation Execution

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `Run` | A complete evaluation execution: dataset × acquisition × mappers × scorers × trials. Built via a builder pattern. | Aligned — "Run" is the common informal term in the domain. | Braintrust uses "Experiment" for a similar concept. The builder enforces required components and type correctness. `.execute()` is the verb (avoids `run.run()`). |
| `Trial` | One execution attempt of one sample: acquire output → map → score. In multi-trial mode, each sample runs N trials to account for non-determinism. | Aligned — "Trial" (Anthropic, Agentrial). More formal than "Run" or "Attempt". | Trial is a runner concept, not a scorer concept. Scorers are pure functions; the Run manages trial repetition. |
| `LLM-as-a-Judge` | Evaluation methodology where an LLM scores another system's output based on a prompt template. The judge LLM receives the input, output, and optionally the reference, and returns a Score. | Aligned — "LLM-as-a-Judge" is the settled domain term (Wikipedia-notable). Used by virtually every eval tool (Langfuse, Arize, DeepEval, Inspect AI). | In this project: `llm_judge()` function (Rust snake_case), `llm-judge` feature gate (Cargo kebab-case). "Model-graded" is a legacy synonym (OpenAI). |
| `pass_at_k` | At least one pass in k trials. Standard metric for non-deterministic evaluation: "did the system succeed at least once?" | Aligned — "pass@k" (HumanEval paper, Chen et al. 2021; Anthropic; EvalPlus). Settled term. | Underscore form is a Rust casing adaptation of "pass@k". |
| `pass_all_k` | All k trials pass. Stricter metric: "does the system succeed every time?" | Aligned — "pass^k" (Anthropic). | More readable than "pass^k". |

### Results

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `RunResult` | The raw output of a completed Run: all trial results for all samples, plus metadata. Contains no computed statistics — stats are derived separately via `.stats()`. | No standard as a distinct raw-data concept. Most tools conflate raw results with computed stats. | Novel. The separation of raw data from interpretation is a deliberate design choice (decisions-log Round 3). |
| `TrialResult` | The result of one trial for one sample: a score (or error) per scorer, plus wall-clock duration. | No standard. | Keyed by scorer name. A trial with partial scorer success (some scorers scored, some errored) counts as scored. |
| `SampleResult` | All trial results for one sample, plus aggregate counts: `trial_count`, `scored_count`, `error_count`. | No standard. | `scored_count` = trials with at least one successful score. `error_count` = trials where ALL scorers errored. |
| `RunMetadata` | Execution metadata for a completed Run: unique ID, timestamps, duration, trial count, scorer definitions, and acquisition mode ("inline" or "observe"). | No standard. | Auto-generated UUID for `run_id`. `score_definitions` persists each scorer's ScoreDefinition for downstream comparison. |

### Statistics

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `RunStats` | Computed statistics from a RunResult: per-scorer aggregated stats across all samples. Produced by `RunResult::stats()`. | No standard as a separate concept. | Novel. Computed, not stored in RunResult. |
| `ScorerStats` | Per-score-type aggregated statistics. An enum matching Score variants: Numeric (mean, stddev, CI, min, max), Binary (pass_rate, pass_at_k, pass_all_k, CI), Label (distribution, mode), Metric (same shape as Numeric). | No standard. | Novel. Errored trials excluded from denominator and reported separately. Wilson CI for Binary; t-distribution CI for Numeric/Metric. |

### Comparison

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `baseline` | The known-good reference run in a comparison. | Aligned — "baseline" (criterion, benchstat). Standard in benchmarking and A/B testing. | |
| `candidate` | The run under evaluation in a comparison. | Common in A/B testing and release engineering. | Most benchmark tools don't name the second side; "candidate" is more explicit than "new" or "test". |
| `Comparison` | The result of comparing two RunResults: per-scorer deltas, significance tests, and per-sample Change directions. | No standard as a distinct type. | Novel. Reports shared scorers, scorers only in baseline, and scorers only in candidate. |
| `ScorerComparison` | Per-scorer comparison within a Comparison: sample-level deltas, aggregate delta (candidate − baseline), p-value, significance flag, and name of statistical test used. | No standard. | Direction-aware: respects ScoreDefinition.direction. |
| `SampleComparison` | Per-sample comparison within a ScorerComparison: score delta and Change direction. | No standard. | |
| `Change` | The direction of difference between baseline and candidate for one sample or aggregate. Variants: `Improved` (candidate is better, direction-aware), `Regressed` (candidate is worse), `Unchanged` (no difference), `Insignificant` (delta exists but not statistically significant), `Incomparable` (different score types, direction mismatch, or insufficient data). | No standard formalization. | Novel. `Incomparable` is a safety variant — prevents misleading comparisons when runs have different configurations. |

### Errors

| Term | Definition | Domain Convention | Notes |
|------|-----------|-------------------|-------|
| `RunBuildError` | Error from invalid Run builder configuration (pre-execution). Variants: `NoDataset`, `NoAcquisition`, `NoScorer`, `EmptyDataset`, `DuplicateSampleIds`, `DuplicateScorerNames`, `MissingSampleIds`. | No standard. | `#[non_exhaustive]`. All caught before any evaluation runs. |
| `RunError` | Error preventing a Run from producing any result. Variants: `Build(RunBuildError)` and `Internal(...)`. | No standard. | `#[non_exhaustive]`. `execute()` returns `Ok(RunResult)` for all evaluation outcomes, including failures. `Err(RunError)` only when no result is possible at all. |
