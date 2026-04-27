# Output Source Naming & 1.0 Public API Design

**Date:** 2026-04-26
**Status:** Draft — ready for review
**Supersedes:** the `Acquisition` umbrella term used through 0.x

## Problem

The current kernel exposes an abstraction called `Acquisition` that covers two genuinely different things:

1. **Active execution** — running a closure, calling an HTTP plugin, or invoking a subprocess to obtain output for a sample.
2. **Passive observation** — reading OTel spans (currently via `evalkit-otel::Observe`), and in the future loading from logs, metrics, fixture files, snapshot stores, or live trace streams.

Two independent transcripts (`transcripts/2026-04-26-acquisition-naming.md`, `transcripts/2026-04-26-acquisition-terminology.md`) reached the same finding: relabeling `Acquisition` cannot fix the awkwardness because **one trait covers two modes**, and no single English noun fits both well. The 1.0 cut is the moment to resolve this by being honest about the split.

## Decision

Adopt a layered API in which:

- The kernel has **one umbrella trait** named after its eval role (supplying output for scoring), with a real verb form.
- The facade has **one method** that accepts any implementation of that trait. The active/passive distinction is communicated by which type the user constructs, not by which builder method they call.
- The active umbrella is a concrete type called `Task`. Passive sources are first-class types in their natural crates (no shared passive umbrella type — the trait is the umbrella).

## Layered API

### Layer 1 — Facade (`Eval`)

One builder method, one transition to the next state.

```rust
// active — closure (blanket impl on the trait)
let result = Eval::new(samples)
    .source(|input: &String| async move { Ok(call_model(input).await?) })
    .scorer(ExactMatch)
    .run()
    .await?;

// active — named adapter
let result = Eval::new(samples)
    .source(Task::http(http_plugin))
    .scorer(ExactMatch)
    .run()
    .await?;

// passive — concrete type from evalkit-otel
let result = Eval::new(samples)
    .source(otel_source)
    .scorer(TrajectoryScorer)
    .run()
    .await?;
```

State machine: `Eval<I, R>` → `.source(...)` → `EvalTask<I, O, R>` → `.scorer(...)` → `EvalRun<I, O, R>` → `.run()`.

The type system enforces "exactly one source + at least one scorer," matching the current `Eval`/`EvalTask`/`EvalRun` machinery — the only change is `acquire` becomes `source`.

### Layer 2 — Kernel builder (`Run::builder()`)

Same method, same trait.

```rust
let run = Run::builder()
    .dataset(dataset)
    .source(my_source)
    .scorer(scorer)
    .trials(3)
    .seed(42)
    .build()?;

let result = run.execute().await?;
```

Facade and kernel have **identical shape** at this layer. Newcomers learn the word once.

### Layer 3 — Trait (`OutputSource`)

```rust
pub trait OutputSource<I, O>: Send + Sync {
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError>;

    async fn produce_with_snapshots(
        &self,
        input: &I,
    ) -> Result<SourceOutput<O>, OutputSourceError> {
        self.produce(input).await.map(SourceOutput::new)
    }

    fn metadata(&self) -> SourceMetadata { SourceMetadata::default() }
}

pub struct SourceOutput<O> {
    pub output: O,
    pub snapshots: Vec<OutputSnapshot<O>>,
}

pub struct OutputSnapshot<O> {
    pub label: String,
    pub output: O,
    pub metadata: HashMap<String, Value>,
}

pub struct SourceMetadata {
    pub mode: &'static str, // "inline" | "task" | "http" | "subprocess" | "observe" | ...
}

pub enum OutputSourceError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    TraceNotFound { correlation_id: String, sample_id: String },
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
    Panicked,
}

// Closures impl OutputSource via a blanket impl, identical to today's Acquisition.
impl<I, O, F, Fut> OutputSource<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: Future<Output = Result<O, OutputSourceError>> + Send,
{ /* ... */ }
```

`produce` is the verb the transcripts identified as the missing piece — it has a real verb form (unlike `evidence`), names the role rather than the operation (unlike `acquire`), and matches `output` end-to-end with `ScorerContext::output`.

### Layer 4 — Concrete adapters

**No shared umbrella type for either side.** The trait is the umbrella; concrete sources are first-class types in their natural crates.

**Active:**

```rust
// evalkit core — for naming an async closure as a source value
pub struct Task<I, O>(/* boxed source */);

impl<I, O> Task<I, O> {
    pub fn from_fn<F, Fut>(f: F) -> Self where /* ... */;
}

impl<I, O> OutputSource<I, O> for Task<I, O> { /* ... */ }

// evalkit-providers — first-class active sources for HTTP and subprocess plugins
pub struct HttpSource { /* ... */ }
impl OutputSource<String, String> for HttpSource { /* ... */ }

pub struct SubprocessSource { /* ... */ }
impl OutputSource<String, String> for SubprocessSource { /* ... */ }
```

`Task` is intentionally narrow — it exists only to *name* a closure-based source for config/reuse/builder use. Closures continue to flow through the trait's blanket impl when no naming is needed. HTTP and subprocess plugins are named first-class types in `evalkit-providers`; they implement `OutputSource` directly and users pass them to `.source(...)` without a wrapper.

This shape was forced by workspace topology — `evalkit-providers` depends on `evalkit`, so `Task::http(plugin)` style constructors (which would require `evalkit` to depend on `evalkit-providers`) are not possible without a cyclic dependency. The result is consistent with the passive side: every concrete source is a first-class type in its natural crate.

**Passive:**

Same pattern — each implementation is a first-class type that impls `OutputSource` directly.

The current `evalkit-otel::Observe` is renamed to `evalkit-otel::OtelObserver` — noun form, indicates the source crate, neutral on live-vs-replay (the user's `(D)` answer in brainstorming: temporal mode is an implementation detail). Future passive sources (`evalkit-fixtures`, log/metric backends, etc.) follow the same pattern: each is a noun-named type in its own crate that impls `OutputSource`.

### Layer 5 — Wire formats

| Format | Before | After |
|---|---|---|
| Plugin protocol JSON | `kind: "acquisition"` | `kind: "source"` |
| TOML config | `[acquisition]` | `[source]` |
| JSONL run-log field | `acquisition_mode` | `source_mode` |
| SQLite column | `acquisition_mode` | `source_mode` |
| JSONL schema version | v1 | v2 |

Pre-1.0 run logs and pre-1.0 plugins are not supported by 1.0+. No migration tool, no dual-read path. Plugin authors rebuild against the new SDK; users who need old run logs keep them in a pre-1.0 binary.

## Symbol rename table

| Before | After |
|---|---|
| `Acquisition` (trait) | `OutputSource` |
| `acquire` (method) | `produce` |
| `acquire_with_snapshots` (method) | `produce_with_snapshots` |
| `AcquisitionError` | `OutputSourceError` |
| `AcquisitionMetadata` | `SourceMetadata` |
| `AcquisitionSnapshot<O>` | `OutputSnapshot<O>` |
| `AcquiredOutput<O>` | `SourceOutput<O>` |
| `Eval::acquire(...)` | `Eval::source(...)` |
| `Run::builder().acquisition(...)` | `Run::builder().source(...)` |
| `acquisition_mode` (metadata field) | `source_mode` |
| `evalkit-otel::Observe` | `evalkit-otel::OtelObserver` |
| `evalkit-providers::HttpAcquisition` | `evalkit-providers::HttpSource` |
| `evalkit-providers::SubprocessAcquisition` | `evalkit-providers::SubprocessSource` |
| `evalkit-providers::AcquisitionPluginRequest` | `evalkit-providers::SourcePluginRequest` |
| `evalkit-providers::AcquisitionPluginResponse` | `evalkit-providers::SourcePluginResponse` |
| `evalkit-providers::AcquisitionPluginConformance` | `evalkit-providers::SourcePluginConformance` |
| `conformance_check_acquisition_plugin` | `conformance_check_source_plugin` |
| `PluginKind::Acquisition` | `PluginKind::Source` |

The `acquisition` *module* in `evalkit/src/` becomes `source.rs`. `current_sample_id` and the task-local plumbing migrate unchanged.

## What stays unchanged

- `Sample`, `Dataset`, `Score`, `RunResult`, `RunMetadata`, `Scorer`, `ScorerSet`, `ScorerContext`, `Mapper`, `compare`, `Comparison`, all stats and result-shape types.
- `ScorerContext::output` — vocabulary already correct.
- Type-state builder shape on `Eval` and `Run::builder()`. Same machinery, renamed methods.
- `evalkit-runtime` boundary: the `compile_fail` doctests in `evalkit/src/lib.rs` continue to enforce that runtime symbols don't leak into the root crate.

## Pedagogical compensation

The facade's single `.source(...)` method does not advertise active/passive at the call site. To compensate, a short "Choosing a Source" section in the docs presents the canonical patterns:

> Most evals start with a closure or `Task::from_fn(...)`.
> To evaluate via HTTP or subprocess plugins, construct an `HttpSource` or `SubprocessSource` from `evalkit-providers` and pass it to `.source(...)`.
> To evaluate an already-instrumented system, construct a passive source (e.g., `OtelObserver` from `evalkit-otel`) and pass it to `.source(...)`.

Crate-level rustdoc on `evalkit-otel` (and any future `evalkit-fixtures`, etc.) lists the passive types it exports.

## Sequencing

Single PR series, no deprecation window. Pre-1.0 has no stability promise, so the rename is a clean break:

1. Rename trait, methods, structs, errors, metadata, snapshot types, and the `acquisition` module across the workspace.
2. Rename facade method from `.acquire(...)` to `.source(...)`. Rename kernel builder method from `.acquisition(...)` to `.source(...)`.
3. Update plugin protocol JSON, TOML config keys, JSONL schema, and SQLite column names. Bump schema and protocol versions.
4. Update bundled plugins, CLI, examples, docs, and tests.
5. Cut 1.0.

Users on 0.x update at call sites; they pin the old version if they need to defer.

## 1.0 stability contract

The following symbols are semver-stable starting at 1.0:

- `OutputSource`, `Task`, `Eval`, `EvalTask`, `EvalRun`, `Run`, `RunBuilder` (and its type-state stages), `Sample`, `Dataset`, `Scorer`, `ScorerContext`, `Score`, `ScoreDefinition`, `RunResult`, `RunMetadata`, `Comparison`, `compare`.
- Wire formats: JSONL run-log v2 schema, SQLite schema, plugin protocol vN.

Out of stability scope (matches existing boundary contract):

- `evalkit-runtime` internals (executors, sinks, samplers, sharding, scrubbers).
- Concrete passive source types in `evalkit-otel` and other adapter crates — these can evolve per their own crate's semver.

## Tradeoff (recorded for posterity)

This is a clean break at 1.0 — no compat shim. ~41 source files touched, three wire formats versioned. Pre-1.0 binaries, run logs, and plugins are not supported by 1.0+. The cost buys:
- A facade and kernel that share one word.
- A trait verb (`produce`) that reads correctly at every implementor and call site.
- An honest API — no single noun is forced to mean both "run something" and "read what already happened."
- Vocabulary alignment with the rest of the eval domain (`output`, `source`, `task`).

Deferred alternatives considered and rejected:

- **Keep `Acquisition`, sharpen the glossary.** Cheaper. Rejected because 1.0 is the only safe moment to break the wire formats and the verb.
- **Layered rename (`task` on facade, `EvidenceSource` underneath, wire formats unchanged).** Adds a second vocabulary without resolving the underlying tension. Rejected.
- **Two facade methods (`.task(...)` + `.observation(...)`).** Considered (Framing 1 in brainstorming). Rejected in favor of single-method simplicity once it became clear the facade and kernel can share one shape.

## Open follow-ups (not blocking this spec)

- Names for additional passive source types as new ones are added (`evalkit-fixtures`, log/metric backends). Convention: noun form, indicates source/crate, neutral on causation and timing.
- Whether to provide a thin `ext` module on `OutputSource` analogous to `ScorerExt` (composability primitives). Out of scope for naming; consider during implementation.
