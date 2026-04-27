# Kernel Output API Redesign (2.0)

**Date:** 2026-04-27
**Status:** Draft — ready for review
**Supersedes:** the 1.0 `OutputSource` shape shipped in `2026-04-26-output-source-naming-design.md`
**Schema impact:** breaking. Bumps `RUN_RESULT_SCHEMA_VERSION`.

## Problem

A multi-agent review of the kernel's output surface (`evalkit/src/source.rs`, `run.rs`, `scorer.rs`, `scorer_context.rs`, `score.rs`, `mapper.rs`, `task.rs`) and a cross-framework comparison against Inspect AI, Promptfoo, DeepEval, Braintrust, OpenAI Evals, and LangSmith surfaced one structural finding and several smaller ones:

- **The output API is bifurcated.** `OutputSource` defines `produce` and `produce_with_snapshots`, but the kernel `Run` only ever calls `produce` (`run.rs:236-246`). Snapshots are dead in the batch path; their `metadata` field is serialized but never consumed (the server even strips it during persistence). Users who carefully implement `produce_with_snapshots` find their work silently discarded.
- **The output envelope is too thin.** `produce` returns bare `O`. Every peer framework returns a richer envelope carrying tokens, cost, latency, and metadata (Inspect's `ModelOutput`, Promptfoo's `ProviderResponse`, DeepEval's `LLMTestCase`, LangSmith's `Run`). evalkit can't capture agent-side resources today because there's nowhere to put them on the return path. `SampleResult.token_usage` and `cost_usd` are populated only from scorer-side `ScorerResources`.
- **Reasoning is locked to numeric scores.** `Score::Structured { score: f64, reasoning, metadata }` forces a numeric primary score (`score.rs:10-14`). A binary or label judge with a rationale either keeps the variant and loses the rationale, or uses `Structured` with `score=1.0` and loses the binary semantics in `RunStats`.
- **`ScorerContext` cannot expose run-scoped facts.** Scorers that want determinism (the `seed`), cooperative cancellation, a cost ceiling, or sibling-score visibility all have to invent side channels (`run.rs:100` stores seed but never threads it to scorers). The current type forces composition tricks like `.then()` chaining for what should be one trial-local state read.
- **`RunStats` silently drops mixed-variant scorers.** `stats.rs:209` collapses any scorer whose trials emit different `Score` variants into `Mixed`, then `finish` returns `None` (`stats.rs:277`), and `filter_map` drops it. The run completes successfully and the scorer simply isn't in `RunStats.scorer_stats`. No error, no warning.
- **Layering leaks.** `OutputSourceError::TraceNotFound { correlation_id, sample_id }` is OTel-shaped. `current_sample_id` task-local has only one consumer, `OtelObserver` in `evalkit-otel`. Both live in the kernel.
- **`SourceMetadata` is dead weight.** A struct around one `&'static str`, immediately converted to `String` for `RunMetadata.source_mode`.
- **Four near-identical mapper executors** (`RawRunExecutor`, `OutputMappedRunExecutor`, `ReferenceMappedRunExecutor`, `FullyMappedRunExecutor` — `run.rs:1036-1136`) duplicate the same pattern with one optional mapper varying.
- **`OutputSourceError`** doesn't signal retry semantics. `Panicked` carries no payload.
- **`Task::from_fn` forces `I: Clone + Send + Sync + 'static`** while the blanket closure impl forces neither. Two paths with different bounds.

User direction: breaking changes are acceptable. Bundle the schema-bumping work into a single 2.0 release rather than dribbling four schema events over the next year.

## Decision

A two-phase 2.0 redesign:

- **Phase 1 — mechanical cleanups.** Internal extractions and refactors. No conceptual shift, no schema impact, can ship as 1.x patches if desired.
- **Phase 2 — schema redesign.** One coordinated semver event covering the output envelope, reasoning, scorer context, and stats fixes. Bumps `RUN_RESULT_SCHEMA_VERSION`.

## Phase 1 — mechanical cleanups

### 1.1 Move snapshots to `evalkit-runtime`

Move `OutputSnapshot<O>`, `SourceOutput<O>`, and `OutputSource::produce_with_snapshots` out of the kernel and into `evalkit-runtime` alongside `PullExecutor::streaming_string_scoring` (the only consumer). The kernel `OutputSource` trait keeps one method, with the bare-`O` return shape until Phase 2 changes it:

```rust
// evalkit/src/source.rs (kernel — after Phase 1, before Phase 2)
pub trait OutputSource<I, O>: Send + Sync {
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError>;
    fn metadata_mode(&self) -> &'static str { "inline" }
}
```

The streaming/snapshot extension trait lives in runtime:

```rust
// evalkit-runtime/src/lib.rs
pub trait SnapshotSource<I, O>: OutputSource<I, O> {
    async fn produce_with_snapshots(&self, input: &I) -> Result<SourceOutput<O>, OutputSourceError>;
}
```

`PullExecutor` checks for `SnapshotSource` via `dyn-clone`-style downcast or a separate `Box<dyn SnapshotSource>` slot — implementation detail decided in the plan.

### 1.2 Move OTel concerns out of the kernel

- Remove `OutputSourceError::TraceNotFound { correlation_id, sample_id }` from the kernel enum. `OtelObserver` produces this case as `OutputSourceError::ExecutionFailed(Box::new(OtelTraceNotFound { ... }))` instead, with the OTel-specific error type defined in `evalkit-otel`.
- Remove `current_sample_id` task-local and `with_current_sample_id` helper from `evalkit/src/source.rs`. Move both to `evalkit-otel`. The kernel's `Run` no longer wraps `produce_output` in this scope.

### 1.3 Drop the `SourceMetadata` struct

Replace with a method on the trait:

```rust
fn metadata_mode(&self) -> &'static str { "inline" }
```

`RunMetadata.source_mode: String` is populated by calling `source.metadata_mode().to_string()` at build time. Same expressiveness, one less type.

### 1.4 Collapse the four mapper executors into one

A single `MappedRunExecutor` holding `Option<Box<dyn Mapper<O, O2>>>` and `Option<Box<dyn Mapper<R, R2>>>` replaces all four variants. Apply each mapper if present, otherwise pass through. The type-state machine on `RunBuilder` (Unmapped/Mapped) remains and continues to enforce "exactly one mapper of each kind, set before build" at compile time. Removes ~100 lines of near-duplicate code in `run.rs`.

### 1.5 `OutputSourceError` polish

```rust
#[non_exhaustive]
pub enum OutputSourceError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
    Panicked(String),  // was payload-free
}

impl OutputSourceError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::BackendUnavailable(_) | Self::Timeout(_))
    }
}
```

`Panicked` extracts the panic message via `std::panic::AssertUnwindSafe` + downcast to `&str` / `String` in `run.rs:212-215`. The kernel doesn't gain a retry policy — `is_retryable` is a hint a runtime middleware can match on.

## Phase 2 — schema redesign

### 2.1 `OutputSource::produce` returns `ProductionOutput<O>`

```rust
#[non_exhaustive]
pub struct ProductionOutput<O> {
    pub output: O,
    pub usage: Option<TokenUsage>,
    pub cost_usd: Option<f64>,
    pub latency: Option<Duration>,
    pub metadata: HashMap<String, Value>,
}

impl<O> ProductionOutput<O> {
    pub fn new(output: O) -> Self { /* all None / empty */ }
    pub fn with_usage(mut self, usage: TokenUsage) -> Self { ... }
    pub fn with_cost_usd(mut self, cost_usd: f64) -> Self { ... }
    pub fn with_latency(mut self, latency: Duration) -> Self { ... }
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self { ... }
}
```

The blanket closure impl handles the common case ergonomically — a closure returning `Result<O, OutputSourceError>` continues to work via:

```rust
impl<I, O, F, Fut> OutputSource<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: Future<Output = Result<O, OutputSourceError>> + Send,
{
    async fn produce(&self, input: &I) -> Result<ProductionOutput<O>, OutputSourceError> {
        self(input).await.map(ProductionOutput::new)
    }
}
```

A second blanket impl for closures that return the rich envelope directly is added in parallel:

```rust
impl<I, O, F, Fut> OutputSource<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: Future<Output = Result<ProductionOutput<O>, OutputSourceError>> + Send,
{
    async fn produce(&self, input: &I) -> Result<ProductionOutput<O>, OutputSourceError> {
        self(input).await
    }
}
```

(Note: Rust's coherence rules require these to be *the same* impl with conditional logic, or a marker-based split. Implementation strategy decided in the plan.)

`Run::execute_trial` reads `usage`, `cost_usd`, `latency` from `ProductionOutput` and folds them into the trial's `ScorerResources`-equivalent accumulator (renamed below). `metadata` flows into `TrialResult.source_metadata: HashMap<String, Value>` for downstream consumption.

**Explicitly excluded from the envelope:** `cache_hit`, `stop_reason`, `model_id`, `choices`. These are chat-completion-shaped and don't generalize across HTTP, subprocess, OTel-observe, or JSONL replay sources. Users who need them put them in `metadata`. A chat-shaped extension type can land later in a domain-specific crate.

### 2.2 Reasoning moves out of `Score`

```rust
// evalkit/src/run_result.rs (after)
pub struct TrialResult {
    pub scores: HashMap<String, ScoredEntry>,
    pub duration: Duration,
    pub trial_index: usize,
    pub source_metadata: HashMap<String, Value>,  // from ProductionOutput.metadata
}

pub struct ScoredEntry {
    pub result: Result<Score, ScorerError>,
    pub reasoning: Option<String>,
    pub metadata: HashMap<String, Value>,
}
```

Scorers populate via an extended `ScoreOutcome`:

```rust
// evalkit/src/scorer.rs (after)
#[non_exhaustive]
pub struct ScoreOutcome {
    pub score: Score,
    pub resources: ResourceUsage,    // see 2.5 for the rename from ScorerResources
    pub reasoning: Option<String>,
    pub metadata: HashMap<String, Value>,
}
```

`Score::Structured { score: f64, reasoning, metadata }` is **removed**. Its uses split:
- Numeric score with rationale → `Score::Numeric(...)` + `ScoreOutcome::reasoning`.
- Binary judge with rationale → `Score::Binary(...)` + `ScoreOutcome::reasoning`.
- Label classifier with rationale → `Score::Label(...)` + `ScoreOutcome::reasoning`.
- Pure structured numeric breakdown without reasoning → `Score::Numeric(value)` + `ScoreOutcome::metadata: { "breakdown": ... }`.

`Score`'s remaining variants: `Numeric(f64)`, `Binary(bool)`, `Label(String)`, `Metric { name, value, unit }`. Stats keep working unchanged because the variants are clean.

### 2.3 `ScorerContext` gains direct fields

```rust
#[non_exhaustive]
pub struct ScorerContext<'a, I, O, R = ()> {
    pub run_id: &'a str,
    pub sample_id: &'a str,
    pub trial_index: usize,
    pub seed: Option<u64>,                                     // NEW
    pub cancel: &'a CancellationToken,                         // NEW
    pub budget: Option<&'a Budget>,                            // NEW
    pub previous_scores: &'a HashMap<String, Score>,           // NEW
    pub metadata: &'a HashMap<String, Value>,                  // sample metadata
    pub input: &'a I,
    pub output: &'a O,
    pub reference: Option<&'a R>,
}
```

- `seed`: `Run::execute_trial` passes `self.seed` directly. Scorers and providers that support seeded sampling read it.
- `cancel`: a tokio-util `CancellationToken` (already an indirect dep). Scorers running long judges check `ctx.cancel.is_cancelled()` cooperatively; the runner triggers cancellation when a sample timeout fires or a budget is exceeded.
- `budget`: a `Budget` type living in the kernel:
    ```rust
    pub struct Budget {
        pub max_cost_usd: Option<f64>,
        pub max_tokens: Option<u64>,
    }
    impl Budget {
        pub fn remaining_cost(&self) -> Option<f64> { ... }
        pub fn would_exceed(&self, additional: ResourceUsage) -> bool { ... }
    }
    ```
    The runner threads a per-sample or per-run budget; scorers can short-circuit instead of paying for an over-budget call.
- `previous_scores`: `ScorerSet` and the run executor populate this with the *successful* results from earlier scorers in the same trial. Failed prior scorers do not appear in this map; if a scorer needs to react to upstream failure, it observes the absence of the expected key. Lets a scorer say "if `cheap_classifier` flagged risky, run this expensive judge; otherwise skip." Today that shape requires `.then()` chaining at the builder layer, which still pays for the cheap call upfront and doesn't let the downstream scorer *read* the upstream score.

Scorer execution order within a `ScorerSet` becomes deterministic (declaration order). Independent scorers across `ScorerSet` boundaries continue to run in parallel; visibility is per-set, not per-trial.

### 2.4 `RunStats` surfaces mixed-variant scorers

```rust
pub struct RunStats {
    pub scorer_stats: HashMap<String, ScorerStats>,
    pub mixed_variant_scorers: Vec<String>,    // NEW
    pub total_samples: usize,
    pub trials_per_sample: usize,
    pub total_trials_executed: usize,
    pub total_errors: usize,
}
```

`ScorerAccumulator::Mixed` cases are collected into `mixed_variant_scorers` instead of dropped. `RunStats::summary` adds:

```
mixed-variant scorers (dropped from stats): <name>, <name>
```

The run still succeeds — this is a data-integrity warning, not an error — but the user can see it.

### 2.5 Resource model unification

`ScorerResources` is renamed to `ResourceUsage` and used by both the scorer side (carried in `ScoreOutcome`) and the source side (extracted from `ProductionOutput`). `SampleResult.token_usage` and `cost_usd` aggregate from both sides.

```rust
#[non_exhaustive]
pub struct ResourceUsage {
    pub token_usage: TokenUsage,
    pub cost_usd: Option<f64>,
    pub latency: Option<Duration>,
}
```

`SampleResult` gains a small breakdown:

```rust
pub struct SampleResult {
    // ... existing fields ...
    pub source_resources: ResourceUsage,    // NEW (from ProductionOutput)
    pub scorer_resources: ResourceUsage,    // RENAMED from inline accumulation
}
```

`token_usage` and `cost_usd` on `SampleResult` remain as the union (sum) for backward-compatible read paths.

## Migration

### For source authors

**Closure path is unchanged.** A closure returning `Result<O, OutputSourceError>` continues to work via the blanket impl; the runner wraps it in `ProductionOutput::new` automatically. No source-author migration unless they want to surface usage.

**To surface usage from an agent shim:**

```rust
let task = Task::from_fn(|input: &String| async move {
    let response = call_model(input).await?;
    let usage = TokenUsage {
        input: response.tokens_in,
        output: response.tokens_out,
        cache_read: response.cache_read,
        cache_write: response.cache_write,
    };
    Ok(ProductionOutput::new(response.text)
        .with_usage(usage)
        .with_cost_usd(response.cost)
        .with_latency(response.elapsed))
});
```

### For scorer authors

Scorers that emit `Score::Numeric`, `Score::Binary`, `Score::Label`, `Score::Metric` are unchanged. Scorers using `Score::Structured` migrate to base variant + `ScoreOutcome::reasoning`:

```rust
// before
Ok(Score::Structured { score: 1.0, reasoning: "matches".into(), metadata: json!({}) })

// after
Ok(ScoreOutcome::new(Score::Binary(true)).with_reasoning("matches"))
```

The default `score()` method continues to return `Result<Score, ScorerError>`; `score_with_resources()` continues to be where reasoning and resources are populated.

`ScorerContext` field additions don't break implementors because the struct is `#[non_exhaustive]` — scorers construct it via `ScorerContext::new` / `with_scope` which remain stable; new fields default to `None` / empty in those constructors.

### For run-log readers

Schema bump. `RUN_RESULT_SCHEMA_VERSION` increments. JSONL readers built against 1.x fail loud (the schema header line mismatches). The migration is a one-shot transform documented alongside the version bump.

`evalkit-server`'s SQLite store needs a migration that reads old-schema rows and rewrites them to new shape — `Score::Structured` cases split into base score + reasoning columns. Documented in the plan.

## Schema version

- `RUN_RESULT_SCHEMA_VERSION` bumps from `"2"` to `"3"`. (The constant is currently `"2"` per `evalkit/src/schema.rs:3` despite the crate's 1.0 release tag — schema versions and crate versions are independent per `docs/stability.md`.)
- `evalkit::write_jsonl` emits the schema-version header line as a strict requirement (already specified in the roadmap; verify or harden in the plan).
- `evalkit::read_jsonl` rejects mismatched schema with a clear error.

## Decisions deferred

- **`Sample<I, R>` multi-slot.** Use trait-bound pattern (`HasContexts`, `HasTrajectory`) in `evalkit-scorers-rag` and `evalkit-scorers-agent`. No kernel change.
- **Multimodal types.** Future `evalkit-multimodal` crate. Out of scope.
- **Per-scorer-target output mappers.** Current global `map_output` is sufficient for now.
- **Trial-level parallelism inside a sample.** Additive option on `Run`; defer until coding-agent suite needs it.
- **Richer statistical tests** (Wilcoxon, McNemar, permutation, effect sizes). Additive to `Comparison` and `RunStats`; defer.
- **Streaming `Dataset`.** Additive trait, defer.
- **Chat-completion-shaped fields** (`cache_hit`, `stop_reason`, `model_id`, `choices`). Out of scope for the kernel envelope. May land later in a domain crate.

## Open questions for the plan

1. **Closure blanket impls and coherence.** Two return shapes (`Result<O, _>` and `Result<ProductionOutput<O>, _>`) need either a marker-based dispatch or a single impl that handles both via conversion. Decide during plan writing.
2. **`previous_scores` semantics across `ScorerSet` boundaries.** Within one set: declaration order, full visibility. Across multiple sets attached to one `Run`: no cross-set visibility (each set is independent). Confirm this matches users' mental model.
3. **`Budget` enforcement layer.** Kernel-level (runner enforces, scorers can't over-spend) vs advisory-only (scorers check and decide). Recommend advisory-only for the kernel; runtime middleware can enforce.
4. **`CancellationToken` source.** Use `tokio_util::sync::CancellationToken` (adds dep to kernel) vs roll a minimal in-house type. Recommend the tokio-util one — already a transitive dep via tokio runtime.
5. **JSONL schema header line.** Verify it's currently emitted; if not, add as part of this release.
6. **Migration tool.** Ship a `evalkit migrate-runlog` CLI subcommand to upgrade 1.x JSONL files? Or document the manual transform? Recommend the subcommand.

## Out of scope

This spec covers the kernel and schema. Downstream effects on `evalkit-providers`, `evalkit-scorers-llm`, `evalkit-scorers-rag`, `evalkit-cli`, `evalkit-server`, `evalkit-otel`, and the polyglot plugin protocol are all consequences of these decisions but their implementation lives in the plan that follows this spec.
