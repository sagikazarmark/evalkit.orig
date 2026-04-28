# Kernel Output API 2.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the bundled 2.0 redesign of the kernel output surface specified in `docs/superpowers/specs/2026-04-27-kernel-output-api-2.0-design.md` — `ProductionOutput<O>` envelope, reasoning on `ScoredEntry`, richer `ScorerContext`, mixed-variant surfacing, and Phase 1 mechanical cleanups.

**Architecture:** Two phases. Phase 1 cleans up dead/leaked surface in the kernel without changing the wire format (5 internal refactors). Phase 2 is the schema event — new envelope, restructured trial/sample shapes, new context fields, schema bump from `"2"` to `"3"`. Phase 3 propagates downstream to provider/scorer/server crates and ships the JSONL migration tool.

**Tech Stack:** Rust 2024 edition, tokio, serde, futures. New dep added: `tokio-util` (for `CancellationToken`).

**Phase boundaries are review checkpoints.** After Phase 1 the workspace should compile and all existing tests pass. After Phase 2 the kernel and core crates compile but downstream crates may not — the schema bump requires Phase 3 to land before a release. After Phase 3 everything builds and a 2.0 release candidate is ready.

---

## File Structure

### Kernel (`evalkit/src/`)

| File | Change |
|---|---|
| `source.rs` | `OutputSource` trait returns `ProductionOutput<O>`; drop `SourceMetadata` struct, `OutputSnapshot`, `SourceOutput`, `produce_with_snapshots`, `current_sample_id`, `with_current_sample_id`; drop `OutputSourceError::TraceNotFound`; add `Panicked(String)` payload, `is_retryable()`; add `ProductionOutput<O>` |
| `run.rs` | Collapse 4 mapper executors into 1; consume `ProductionOutput<O>`; populate `source_resources` accumulator; thread `seed`, `cancel`, `budget`, `previous_scores` into `ScorerContext`; pass panic message into `Panicked(String)` |
| `run_result.rs` | Rename `ScorerResources` → `ResourceUsage`; add `latency` field; add `ScoredEntry` (replaces `Result<Score, ScorerError>` map values); add `TrialResult.source_metadata`; add `SampleResult.source_resources` and `scorer_resources` |
| `scorer.rs` | Extend `ScoreOutcome` with `reasoning`, `metadata`; `resources` field uses `ResourceUsage` |
| `scorer_context.rs` | Add `seed`, `cancel`, `budget`, `previous_scores` fields |
| `score.rs` | Remove `Score::Structured` variant |
| `stats.rs` | Drop `Score::Structured` handling; route `Mixed` accumulator into `RunStats.mixed_variant_scorers` |
| `comparison.rs` | Drop `Score::Structured` handling |
| `scorer_ext.rs` | Drop `Score::Structured` handling in `ignore_reference` and weighted score extraction |
| `scorer_set.rs` | Sequential execution within set; populate `previous_scores` for downstream scorers |
| `schema.rs` | Bump `RUN_RESULT_SCHEMA_VERSION` from `"2"` to `"3"` |
| `jsonl.rs` | (no logic change; the version constant flows through) |
| `task.rs` | Update to compose with envelope |
| `lib.rs` | Update re-exports; remove dropped items, add new ones |
| `eval.rs` | (likely no change; pure pass-through) |
| `Cargo.toml` | Add `tokio-util = "0.7"`; bump version to `2.0.0` |

### Runtime (`evalkit-runtime/src/`)

| File | Change |
|---|---|
| `lib.rs` (new module `snapshots.rs` or extend) | Add `OutputSnapshot<O>`, `SourceOutput<O>`, `SnapshotSource` extension trait |
| `executor.rs` | `PullExecutor::streaming_string_scoring` consumes `SnapshotSource` instead of kernel `OutputSource::produce_with_snapshots` |
| `Cargo.toml` | Bump version to `2.0.0` |

### OTel (`evalkit-otel/src/`)

| File | Change |
|---|---|
| `lib.rs` | Add `OtelTraceNotFound` error type; add `current_sample_id` task-local + `with_current_sample_id` helper; `OtelObserver` produces `OutputSourceError::ExecutionFailed(Box::new(OtelTraceNotFound { ... }))` instead of `TraceNotFound` |

### Server (`evalkit-server/src/`)

| File | Change |
|---|---|
| Storage layer | Migrate v2 → v3 schema for stored runs; adapt to `ScoredEntry`-shaped `TrialResult.scores` and split `source_resources`/`scorer_resources` |
| Migration code | Read v2 row, transform `Score::Structured` into base-variant + reasoning, write v3 |

### CLI (`evalkit-cli/src/`)

| File | Change |
|---|---|
| Subcommand handler | Add `evalkit migrate-runlog --in <path> --out <path>` for v2 → v3 JSONL transform |
| `run`, `diff`, `watch` | Adapt to new `TrialResult` / `SampleResult` field shapes |

### Provider crates

| Crate | Change |
|---|---|
| `evalkit-providers` | `HttpAcquisition`, `SubprocessAcquisition`, `SubprocessScorer` updated for `ProductionOutput<O>` return |
| `evalkit-scorers-llm` | All `Score::Structured` constructions migrate to base variant + `ScoreOutcome::reasoning`; `LlmJudge` and `g_eval` updated |
| `evalkit-scorers-text`, `-rag`, `-embed`, `-redteam` | No-op or minor type-alias updates |
| `evalkit-exporters-langfuse` | Adapt to new `TrialResult.scores` shape |

---

## Phase 1 — mechanical cleanups

### Task 1: Add `is_retryable()` and `Panicked(String)` payload

**Files:**
- Modify: `evalkit/src/source.rs:84-93`, `evalkit/src/source.rs` (impl block)
- Modify: `evalkit/src/run.rs:182-216` (panic capture in `execute_trial`)
- Test: `evalkit/src/source.rs` (test module at end of file)

- [ ] **Step 1: Write the failing tests for `is_retryable` and `Panicked(String)`**

Append to `evalkit/src/source.rs` test module:

```rust
#[test]
fn is_retryable_classifies_known_variants() {
    use std::time::Duration;
    let backend = OutputSourceError::BackendUnavailable(Box::new(TestError("down")));
    let timeout = OutputSourceError::Timeout(Duration::from_secs(1));
    let exec = OutputSourceError::ExecutionFailed(Box::new(TestError("bad")));
    let panicked = OutputSourceError::Panicked("boom".to_string());

    assert!(backend.is_retryable());
    assert!(timeout.is_retryable());
    assert!(!exec.is_retryable());
    assert!(!panicked.is_retryable());
}

#[test]
fn panicked_carries_message() {
    let err = OutputSourceError::Panicked("agent shim crashed".to_string());
    assert_eq!(
        err.to_string(),
        "output source panicked: agent shim crashed"
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p evalkit --lib source::tests::is_retryable_classifies_known_variants source::tests::panicked_carries_message
```

Expected: compile errors (`Panicked` doesn't take a payload; `is_retryable` doesn't exist).

- [ ] **Step 3: Update the enum and add `is_retryable`**

Edit `evalkit/src/source.rs`:

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum OutputSourceError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    TraceNotFound {
        correlation_id: String,
        sample_id: String,
    },
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
    Panicked(String),
}

impl OutputSourceError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::BackendUnavailable(_) | Self::Timeout(_))
    }
}

impl Display for OutputSourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutionFailed(err) => write!(f, "output source execution failed: {err}"),
            Self::TraceNotFound { correlation_id, sample_id } => write!(
                f,
                "no spans found for correlation_id `{correlation_id}` and sample_id `{sample_id}`"
            ),
            Self::BackendUnavailable(err) => write!(f, "trace backend unavailable: {err}"),
            Self::Timeout(duration) => write!(f, "output source timed out after {duration:?}"),
            Self::Panicked(message) => write!(f, "output source panicked: {message}"),
        }
    }
}
```

(Note: `TraceNotFound` is dropped in Task 4. Keep it here for now to keep this task isolated.)

- [ ] **Step 4: Update `run.rs` to extract panic message**

Edit `evalkit/src/run.rs` around line 212. Replace the panic-arm `OutputSourceError::Panicked` constructor:

```rust
            Err(payload) => FlattenedTrial {
                scores: source_failure_scores(
                    &self.definitions,
                    OutputSourceError::Panicked(panic_message(payload)),
                ),
                resources: ScorerResources::default(),
            },
```

Add helper near `scorer_panic_scores`:

```rust
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "<non-string panic payload>".to_string()
}
```

Same change for the inner `catch_unwind` around `executor.execute` — `ScorerError::internal(ScorerPanicError)` already covers scorer panics, so leave it. Only `produce_output` panic feeds `OutputSourceError::Panicked`.

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cargo test -p evalkit --lib source::tests::
cargo test -p evalkit
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/source.rs evalkit/src/run.rs
git commit -m "$(cat <<'EOF'
feat(kernel): add OutputSourceError::is_retryable and panic payload

Panicked carries the panic message instead of being a unit variant;
is_retryable() is a hint a runtime middleware can match on.

Per docs/superpowers/specs/2026-04-27-kernel-output-api-2.0-design.md
section 1.5.
EOF
)"
```

---

### Task 2: Drop `SourceMetadata` struct, replace with trait method

**Files:**
- Modify: `evalkit/src/source.rs:20-30, 76-81, 122-133`
- Modify: `evalkit/src/task.rs:97-100`
- Modify: `evalkit/src/run.rs:272` (where `source.metadata().mode` is read)
- Modify: `evalkit/src/lib.rs:110-112` (remove `SourceMetadata` from re-exports)
- Modify: `evalkit-otel/src/lib.rs` (search for `SourceMetadata`)
- Modify: `evalkit-providers/src/lib.rs` (search for `SourceMetadata`)
- Modify: `evalkit-runtime/src/lib.rs` (search for `SourceMetadata`)

- [ ] **Step 1: Write a test that exercises the new method shape**

Append to `evalkit/src/source.rs` test module:

```rust
#[test]
fn metadata_mode_default_is_inline() {
    struct Bare;
    impl OutputSource<String, String> for Bare {
        async fn produce(&self, _input: &String) -> Result<String, OutputSourceError> {
            Ok(String::new())
        }
    }
    let bare = Bare;
    assert_eq!(bare.metadata_mode(), "inline");
}
```

- [ ] **Step 2: Run test, confirm it fails**

```bash
cargo test -p evalkit --lib source::tests::metadata_mode_default_is_inline
```

Expected: `no method named metadata_mode`.

- [ ] **Step 3: Replace the trait method**

In `evalkit/src/source.rs`, delete the `SourceMetadata` struct and `Default`/`mode` impls. Replace `metadata()` with `metadata_mode()`:

```rust
#[allow(async_fn_in_trait)]
pub trait OutputSource<I, O>: Send + Sync {
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError>;

    async fn produce_with_snapshots(&self, input: &I) -> Result<SourceOutput<O>, OutputSourceError> {
        self.produce(input).await.map(SourceOutput::new)
    }

    fn metadata_mode(&self) -> &'static str { "inline" }
}
```

(`SourceOutput`, `OutputSnapshot`, and `produce_with_snapshots` get moved out in Task 5. Keep them for now.)

- [ ] **Step 4: Update `Task` and the `run.rs` reader**

`evalkit/src/task.rs:97-100`:

```rust
impl<I, O> OutputSource<I, O> for Task<I, O>
where
    I: Send + Sync,
    O: Send + Sync,
{
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError> {
        (self.produce)(input).await
    }

    fn metadata_mode(&self) -> &'static str {
        self.mode
    }
}
```

`evalkit/src/run.rs:272` (in `RunBuilderWithDataset::source`):

```rust
        let source_mode = source.metadata_mode();
```

- [ ] **Step 5: Update extension crates**

Search and replace `metadata()` returning `SourceMetadata` with `metadata_mode()` returning `&'static str` in:
- `evalkit-otel/src/lib.rs` (look for `impl OutputSource for OtelObserver`)
- `evalkit-providers/src/lib.rs` (look for `HttpAcquisition`, `SubprocessAcquisition`)
- `evalkit-runtime/src/lib.rs` (any source impls)

For each, replace:

```rust
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata { mode: "observe" }
    }
```

with:

```rust
    fn metadata_mode(&self) -> &'static str { "observe" }
```

(or whatever the existing mode string was).

- [ ] **Step 6: Drop `SourceMetadata` from kernel re-exports**

`evalkit/src/lib.rs:110-112`:

```rust
pub use source::{
    OutputSource, OutputSourceError, OutputSnapshot, SourceOutput,
};
```

(Remove `SourceMetadata`.)

- [ ] **Step 7: Run tests**

```bash
cargo test --workspace
```

Expected: all green.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(kernel): replace SourceMetadata struct with metadata_mode method

The struct carried one &'static str field that was immediately read
out and converted to String. A trait method is the same expressiveness
with one less type.

Per spec section 1.3.
EOF
)"
```

---

### Task 3: Collapse the four mapper executors into one

**Files:**
- Modify: `evalkit/src/run.rs:1032-1136` (the four executor impls)
- Modify: `evalkit/src/run.rs:797-913` (the four `build` impls dispatching to executors)

- [ ] **Step 1: Write a test that exercises both mapped and unmapped paths**

Append to `evalkit/src/run.rs` test module (or add a new test module if needed):

```rust
#[tokio::test(flavor = "current_thread")]
async fn mapper_executor_handles_no_mappers() {
    let dataset = Dataset::new(vec![
        Sample::builder("x".to_string()).id("s1").build().unwrap(),
    ]);
    let run = Run::builder()
        .dataset(dataset)
        .source(|input: &String| {
            let input = input.clone();
            async move { Ok::<_, OutputSourceError>(input) }
        })
        .scorer(crate::tests::ContainsScorer)
        .build()
        .unwrap();
    let result = run.execute().await.unwrap();
    assert_eq!(result.samples.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn mapper_executor_applies_output_mapper() {
    use crate::Mapper;
    struct ToLen;
    impl Mapper<String, usize> for ToLen {
        fn map(&self, input: &String) -> Result<usize, crate::MapError> {
            Ok(input.len())
        }
    }
    struct LenScorer;
    impl Scorer<String, usize, String> for LenScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, usize, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Numeric(*ctx.output as f64))
        }
        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("len")
        }
    }

    let dataset = Dataset::new(vec![
        Sample::builder("hello".to_string())
            .id("s1")
            .reference(String::new())
            .build()
            .unwrap(),
    ]);
    let run = Run::builder()
        .dataset(dataset)
        .source(|input: &String| {
            let input = input.clone();
            async move { Ok::<_, OutputSourceError>(input) }
        })
        .map_output(ToLen)
        .scorer(LenScorer)
        .build()
        .unwrap();
    let result = run.execute().await.unwrap();
    let score = &result.samples[0].trials[0].scores["len"];
    assert!(matches!(score, Ok(Score::Numeric(v)) if (v - 5.0).abs() < f64::EPSILON));
}
```

(Adjust imports as needed.)

- [ ] **Step 2: Run tests to verify they pass on existing code (regression baseline)**

```bash
cargo test -p evalkit --lib run::tests::mapper_executor_handles_no_mappers run::tests::mapper_executor_applies_output_mapper
```

Expected: pass (these tests describe current behavior; we want them to continue passing after refactor).

- [ ] **Step 3: Replace the four executors with one**

In `evalkit/src/run.rs`, delete `RawRunExecutor`, `OutputMappedRunExecutor`, `ReferenceMappedRunExecutor`, `FullyMappedRunExecutor` (lines 1036-1136). Replace with:

```rust
struct MappedRunExecutor<I, O, R, O2, R2> {
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    targets: Vec<ScoringTarget<I, O2, R2>>,
}

impl<I, O, R, O2, R2> RunExecutor<I, O, R> for MappedRunExecutor<I, O, R, O2, R2>
where
    O: 'static,
    R: 'static,
    O2: 'static,
    R2: 'static,
{
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(async move {
            // Output mapping: pass through if no mapper, else apply.
            let mapped_output_storage;
            let mapped_output_ref: &O2 = match &self.output_mapper {
                Some(mapper) => match mapper.map(ctx.output) {
                    Ok(value) => {
                        mapped_output_storage = value;
                        &mapped_output_storage
                    }
                    Err(err) => return map_failure_results(&self.targets, err),
                },
                None => {
                    // Safety: when no mapper, O2 == O by builder construction.
                    // Caller invariant from RunBuilder type-state.
                    unsafe { &*(ctx.output as *const O as *const O2) }
                }
            };

            // Reference mapping: same pattern.
            let mapped_reference_storage;
            let mapped_reference_ref: Option<&R2> = match (&self.reference_mapper, ctx.reference) {
                (Some(mapper), Some(reference)) => match mapper.map(reference) {
                    Ok(value) => {
                        mapped_reference_storage = value;
                        Some(&mapped_reference_storage)
                    }
                    Err(err) => return map_failure_results(&self.targets, err),
                },
                (None, Some(reference)) => {
                    Some(unsafe { &*(reference as *const R as *const R2) })
                }
                (_, None) => None,
            };

            let mapped_ctx = ScorerContext {
                run_id: ctx.run_id,
                sample_id: ctx.sample_id,
                trial_index: ctx.trial_index,
                metadata: ctx.metadata,
                input: ctx.input,
                output: mapped_output_ref,
                reference: mapped_reference_ref,
            };

            execute_targets(&self.targets, &mapped_ctx).await
        })
    }
}
```

The `unsafe` casts handle the type-state-enforced "if no mapper, then O2 == O" invariant. The builder type-state guarantees this — the unsafe blocks are correct because the `Unmapped`/`Mapped` flags determine which `build()` arm constructs the executor.

**Alternative if `unsafe` is unwanted:** Instead store both as `Box<dyn FnOnce(&O) -> Result<&O2, MapError>>` adapter closures that for the unmapped case use a transmute helper guarded by a trait bound. The safest readable form is to keep the type-state machine but have it always provide an identity mapper for the unmapped slot. **Decide during implementation; document the choice in `docs/decisions.md`.**

- [ ] **Step 4: Replace the four `build()` impls with one**

Delete the four `RunBuilderWithTargets::build` impls (lines 797-913) and replace with one that constructs `MappedRunExecutor` with `Option`-typed mappers:

```rust
impl<I: 'static, O: 'static, R: 'static, O2: 'static, R2: 'static, OS, RS>
    RunBuilderWithTargets<I, O, R, O2, R2, OS, RS>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError>
    where
        // Trait bounds that make the executor's identity-cast valid in the
        // unmapped case. See decision note in run.rs.
    {
        let this = self.resolved_code_identity().normalized_judge_model_pins();
        let definitions = this.validate()?;

        Ok(Run {
            dataset: this.dataset,
            source: this.source,
            definitions,
            executor: Box::new(MappedRunExecutor {
                output_mapper: this.output_mapper,
                reference_mapper: this.reference_mapper,
                targets: this.targets,
            }),
            trial_count: this.trial_count,
            concurrency: this.concurrency,
            sample_timeout: this.sample_timeout,
            seed: this.seed,
            code_commit: this.code_commit,
            code_fingerprint: this.code_fingerprint,
            judge_model_pins: this.judge_model_pins,
            source_mode: this.source_mode,
        })
    }
}
```

(The type-state machine `Unmapped`/`Mapped` markers can be retained as compile-time guards on `RunBuilderConfigured::scorer` to ensure scorers see the post-mapping types — that part of the machine is independent of the executor count.)

- [ ] **Step 5: Run all tests**

```bash
cargo test --workspace
```

Expected: all green, including the two new tests from Step 1.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/run.rs
git commit -m "$(cat <<'EOF'
refactor(kernel): collapse four mapper executors into one

RawRunExecutor, OutputMappedRunExecutor, ReferenceMappedRunExecutor,
and FullyMappedRunExecutor were near-identical impls differing only in
which optional mapper they applied. Replace with one MappedRunExecutor
holding Option<Box<dyn Mapper>> for each axis.

The RunBuilder type-state machine (Unmapped/Mapped) is retained where
it enforces "scorers see post-mapping types"; the per-executor variant
explosion is removed.

Per spec section 1.4.
EOF
)"
```

---

### Task 4: Move OTel concerns out of the kernel

**Files:**
- Modify: `evalkit/src/source.rs:84-93` (drop `TraceNotFound` variant), `:16-18, :145-154` (drop `current_sample_id` and helper)
- Modify: `evalkit/src/run.rs:228-234` (remove `with_current_sample_id` wrapping)
- Modify: `evalkit/src/lib.rs` (remove `current_sample_id` from compile_fail tests if present)
- Modify: `evalkit-otel/src/lib.rs` (add `OtelTraceNotFound` error type, add `current_sample_id` task-local + helper, update `OtelObserver`)
- Test: `evalkit-otel/src/lib.rs` test module

- [ ] **Step 1: Write a test for the new `OtelTraceNotFound` error type in evalkit-otel**

In `evalkit-otel/src/lib.rs` test module, add:

```rust
#[test]
fn otel_trace_not_found_displays_correlation_and_sample() {
    let err = OtelTraceNotFound {
        correlation_id: "run-1".to_string(),
        sample_id: "s-1".to_string(),
    };
    assert_eq!(
        err.to_string(),
        "no spans found for correlation_id `run-1` and sample_id `s-1`"
    );
}

#[test]
fn current_sample_id_returns_none_outside_scope() {
    assert!(current_sample_id().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn current_sample_id_returns_value_inside_scope() {
    let result = with_current_sample_id("s-42", async {
        current_sample_id()
    }).await;
    assert_eq!(result.as_deref(), Some("s-42"));
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit-otel
```

Expected: `OtelTraceNotFound`, `current_sample_id`, `with_current_sample_id` not defined.

- [ ] **Step 3: Add the OTel-side types**

In `evalkit-otel/src/lib.rs`:

```rust
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use tokio::task_local;

#[derive(Debug, Clone)]
pub struct OtelTraceNotFound {
    pub correlation_id: String,
    pub sample_id: String,
}

impl Display for OtelTraceNotFound {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no spans found for correlation_id `{}` and sample_id `{}`",
            self.correlation_id, self.sample_id
        )
    }
}

impl Error for OtelTraceNotFound {}

task_local! {
    static CURRENT_SAMPLE_ID: String;
}

pub fn current_sample_id() -> Option<String> {
    CURRENT_SAMPLE_ID.try_with(Clone::clone).ok()
}

pub async fn with_current_sample_id<Fut>(sample_id: &str, future: Fut) -> Fut::Output
where
    Fut: Future,
{
    CURRENT_SAMPLE_ID.scope(sample_id.to_string(), future).await
}
```

Update `OtelObserver` (or whatever produces the error today). Search the file for `TraceNotFound` and replace:

```rust
// before:
return Err(OutputSourceError::TraceNotFound {
    correlation_id: correlation_id.to_string(),
    sample_id: sample_id.clone(),
});

// after:
return Err(OutputSourceError::ExecutionFailed(Box::new(OtelTraceNotFound {
    correlation_id: correlation_id.to_string(),
    sample_id: sample_id.clone(),
})));
```

If `OtelObserver` was relying on `current_sample_id` set by the kernel, it now must scope it itself before reading. Search for usages of `evalkit::current_sample_id` and replace with the local one.

- [ ] **Step 4: Drop the kernel-side OTel-shaped concerns**

`evalkit/src/source.rs`:

Delete `task_local! { static CURRENT_SAMPLE_ID: ... }` (lines ~16-18).

Delete `pub fn current_sample_id()` and `pub async fn with_current_sample_id()` (lines ~145-154).

Drop the `TraceNotFound` variant from `OutputSourceError` enum and from the `Display` / `Error::source` impls.

`evalkit/src/run.rs`:

Replace the `with_current_sample_id` wrapping in `produce_output`:

```rust
async fn produce_output(&self, sample: &Sample<I, R>) -> Result<O, OutputSourceError> {
    self.produce_output_inner(&sample.input).await
}
```

(The kernel no longer scopes a sample-id task-local. If a passive source needs sample-id correlation, it sets up its own scope.)

`evalkit/src/lib.rs`: remove the `compile_fail` doc-test for `current_sample_id` if it exists, or update the boundary tests to reflect the new public surface.

- [ ] **Step 5: Update OTel-side OutputSource impls to scope the task-local themselves**

The kernel no longer wraps `produce()` calls in `with_current_sample_id`. If `OtelObserver` (or any passive source) relies on the sample-id task-local, it must establish the scope inside its own `produce` impl using whatever `Sample`-id channel is appropriate. **Recommended approach:** the kernel passes the sample id via `ScorerContext.sample_id` for scorers; for sources, the simplest path is to remove the dependency on the task-local entirely and have `OtelObserver` accept the sample id as part of the input `I` (since it's input-derived correlation, not runtime-injected state). Decide during implementation.

- [ ] **Step 6: Run all tests**

```bash
cargo test --workspace
```

Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor: move OTel concerns out of kernel

Drop OutputSourceError::TraceNotFound (OTel-shaped variant in a generic
enum) and the current_sample_id task-local (sole consumer was
OtelObserver). Both move to evalkit-otel; OtelObserver now wraps its
trace-not-found case in OutputSourceError::ExecutionFailed.

Per spec section 1.2.
EOF
)"
```

---

### Task 5: Move snapshots to `evalkit-runtime`

**Files:**
- Modify: `evalkit/src/source.rs:32-74` (remove `OutputSnapshot`, `SourceOutput`)
- Modify: `evalkit/src/source.rs:122-133` (remove `produce_with_snapshots` from trait)
- Modify: `evalkit/src/lib.rs` (drop `OutputSnapshot`, `SourceOutput` from re-exports)
- Modify: `evalkit-runtime/src/lib.rs` (add `OutputSnapshot`, `SourceOutput`, `SnapshotSource`)
- Modify: `evalkit-runtime/src/executor.rs` (use `SnapshotSource` instead of kernel `produce_with_snapshots`)
- Test: `evalkit-runtime/src/lib.rs` (test module)

- [ ] **Step 1: Add `OutputSnapshot`, `SourceOutput`, `SnapshotSource` to evalkit-runtime**

In `evalkit-runtime/src/lib.rs` (or a new `snapshots.rs` module exported from lib):

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use evalkit::{OutputSource, OutputSourceError};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputSnapshot<O> {
    pub label: String,
    pub output: O,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl<O> OutputSnapshot<O> {
    pub fn new(label: impl Into<String>, output: O) -> Self {
        Self {
            label: label.into(),
            output,
            metadata: HashMap::new(),
        }
    }

    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceOutput<O> {
    pub output: O,
    #[serde(default)]
    pub snapshots: Vec<OutputSnapshot<O>>,
}

impl<O> SourceOutput<O> {
    pub fn new(output: O) -> Self {
        Self { output, snapshots: Vec::new() }
    }

    pub fn with_snapshot(mut self, snapshot: OutputSnapshot<O>) -> Self {
        self.snapshots.push(snapshot);
        self
    }
}

#[allow(async_fn_in_trait)]
pub trait SnapshotSource<I, O>: OutputSource<I, O> {
    async fn produce_with_snapshots(
        &self,
        input: &I,
    ) -> Result<SourceOutput<O>, OutputSourceError>;
}
```

- [ ] **Step 2: Drop snapshot types from kernel**

`evalkit/src/source.rs`: delete `OutputSnapshot<O>`, `SourceOutput<O>` and their impls. Drop `produce_with_snapshots` from the `OutputSource` trait. Final shape:

```rust
#[allow(async_fn_in_trait)]
pub trait OutputSource<I, O>: Send + Sync {
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError>;
    fn metadata_mode(&self) -> &'static str { "inline" }
}
```

`evalkit/src/lib.rs`: remove `OutputSnapshot`, `SourceOutput` from `pub use source::{...}`.

- [ ] **Step 3: Update `PullExecutor` in evalkit-runtime to use `SnapshotSource`**

In `evalkit-runtime/src/executor.rs`, find the call site of `produce_with_snapshots`. Change the source slot in `PullExecutor` from `Box<dyn OutputSource>` to either:
- `Box<dyn SnapshotSource>` (requires sources to implement the new trait), or
- `Box<dyn OutputSource>` + an `Option<Box<dyn SnapshotSource>>` slot for streaming-aware paths.

Recommended: add a separate `streaming_source` field in `PullExecutor`:

```rust
pub struct PullExecutor<I, O, R> {
    source: Box<dyn OutputSource<I, O>>,
    streaming_source: Option<Box<dyn SnapshotSource<I, O>>>,
    // ... rest unchanged
}

impl<I, O, R> PullExecutor<I, O, R> {
    pub fn streaming_source(mut self, source: impl SnapshotSource<I, O> + 'static) -> Self {
        self.streaming_source = Some(Box::new(source));
        self
    }
}
```

`streaming_string_scoring(...)` requires `streaming_source` to be set; otherwise it errors at builder time.

- [ ] **Step 4: Update any in-tree implementors of `produce_with_snapshots`**

Search workspace:

```bash
grep -rn "produce_with_snapshots" /home/laborant/evalkit.orig/
```

Each impl that overrode `produce_with_snapshots` on the kernel trait needs to also `impl SnapshotSource<I, O>` in the runtime crate. For most tests this is a one-line addition.

- [ ] **Step 5: Add a smoke test in evalkit-runtime**

In `evalkit-runtime/src/lib.rs` test module:

```rust
#[tokio::test(flavor = "current_thread")]
async fn snapshot_source_default_returns_output_only() {
    struct Bare;
    impl OutputSource<String, String> for Bare {
        async fn produce(&self, input: &String) -> Result<String, OutputSourceError> {
            Ok(input.clone())
        }
    }
    impl SnapshotSource<String, String> for Bare {
        async fn produce_with_snapshots(
            &self,
            input: &String,
        ) -> Result<SourceOutput<String>, OutputSourceError> {
            Ok(SourceOutput::new(input.clone()))
        }
    }

    let bare = Bare;
    let out = bare.produce_with_snapshots(&"hi".to_string()).await.unwrap();
    assert_eq!(out.output, "hi");
    assert!(out.snapshots.is_empty());
}
```

- [ ] **Step 6: Run all tests**

```bash
cargo test --workspace
```

Expected: green. The kernel `evalkit::OutputSource` trait now has only `produce` + `metadata_mode`. Snapshot users opt into the runtime extension.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor: move snapshots from kernel to evalkit-runtime

OutputSnapshot, SourceOutput, and produce_with_snapshots were defined
in the kernel but only consumed by PullExecutor in evalkit-runtime.
The two-method OutputSource trait (with the rich method dead in the
batch path) was the worst of both worlds. Move snapshots to runtime as
a SnapshotSource extension trait; kernel OutputSource is now one
method.

Per spec section 1.1.
EOF
)"
```

---

**Phase 1 review checkpoint.** At this point:

- Kernel surface is leaner: one `produce` method, no `SourceMetadata` struct, no `OutputSnapshot` / `SourceOutput`, no `current_sample_id`, no `TraceNotFound`.
- `OutputSourceError` carries panic messages and signals retry semantics.
- `Run` has one mapper executor instead of four.
- `evalkit-runtime` owns snapshot types.
- `evalkit-otel` owns the OTel-specific error and task-local.
- All existing tests pass. No schema change.

Workspace should compile and tests pass. **Recommended:** stop here for review before starting Phase 2.

---

## Phase 2 — schema redesign

### Task 6: Rename `ScorerResources` → `ResourceUsage`, add `latency` field

**Files:**
- Modify: `evalkit/src/scorer.rs:3-34` (the `ScorerResources` definition, builder methods, merge)
- Modify: `evalkit/src/run_result.rs` (any field references)
- Modify: `evalkit/src/run.rs` (any references)
- Modify: `evalkit/src/lib.rs` (re-export rename)
- Modify: `evalkit-scorers-llm/src/lib.rs` (uses `ScorerResources` extensively)
- Modify: any other downstream uses

- [ ] **Step 1: Add a test for `ResourceUsage` with latency**

Append to `evalkit/src/scorer.rs` test module:

```rust
#[test]
fn resource_usage_merges_latency() {
    use std::time::Duration;
    let mut a = ResourceUsage::default()
        .latency(Duration::from_millis(100));
    let b = ResourceUsage::default()
        .latency(Duration::from_millis(50));
    a.merge(&b);
    assert_eq!(a.latency, Some(Duration::from_millis(150)));
}

#[test]
fn resource_usage_merges_with_missing_latency() {
    use std::time::Duration;
    let mut a = ResourceUsage::default()
        .latency(Duration::from_millis(100));
    let b = ResourceUsage::default();
    a.merge(&b);
    assert_eq!(a.latency, Some(Duration::from_millis(100)));
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib scorer::tests::resource_usage_merges_latency
```

Expected: `ResourceUsage` not defined.

- [ ] **Step 3: Rename and extend the type**

`evalkit/src/scorer.rs`:

```rust
use std::time::Duration;
use crate::{Score, ScoreDefinition, ScorerContext, ScorerError, TokenUsage};

#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ResourceUsage {
    pub token_usage: TokenUsage,
    pub cost_usd: Option<f64>,
    pub latency: Option<Duration>,
}

impl ResourceUsage {
    pub fn token_usage(mut self, token_usage: TokenUsage) -> Self {
        self.token_usage = token_usage;
        self
    }

    pub fn cost_usd(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }

    pub fn latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }

    pub fn merge(&mut self, other: &Self) {
        self.token_usage.input += other.token_usage.input;
        self.token_usage.output += other.token_usage.output;
        self.token_usage.cache_read += other.token_usage.cache_read;
        self.token_usage.cache_write += other.token_usage.cache_write;

        self.cost_usd = match (self.cost_usd, other.cost_usd) {
            (Some(left), Some(right)) => Some(left + right),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };

        self.latency = match (self.latency, other.latency) {
            (Some(left), Some(right)) => Some(left + right),
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        };
    }
}
```

- [ ] **Step 4: Update `ScoreOutcome` and the `Scorer` trait**

```rust
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScoreOutcome {
    pub score: Score,
    pub resources: ResourceUsage,
}

impl ScoreOutcome {
    pub fn new(score: Score) -> Self {
        Self { score, resources: ResourceUsage::default() }
    }

    pub fn with_resources(mut self, resources: ResourceUsage) -> Self {
        self.resources = resources;
        self
    }
}
```

(`reasoning` and `metadata` fields land in Task 15. Keep `ScoreOutcome` lean for this commit.)

- [ ] **Step 5: Update lib.rs re-export**

`evalkit/src/lib.rs:129`:

```rust
pub use scorer::{ScoreOutcome, Scorer, ScorerMetadata, ResourceUsage};
```

(Drop `ScorerResources`.)

- [ ] **Step 6: Search-and-replace `ScorerResources` → `ResourceUsage`**

```bash
grep -rln "ScorerResources" /home/laborant/evalkit.orig/ --include="*.rs"
```

For each hit (likely `evalkit/src/run.rs`, `evalkit/src/run_result.rs`, `evalkit-scorers-llm/src/lib.rs`, possibly others), replace `ScorerResources` with `ResourceUsage`. The API is identical aside from the new `latency` field and the rename.

`evalkit/src/run_result.rs`: rename references in trial/sample resource accumulation.

`evalkit/src/run.rs`: in `flatten_scores` (line ~1226) and any local accumulator type.

`evalkit-scorers-llm/src/lib.rs`: search for `ScorerResources::new`, `ScorerResources::default`, etc.

- [ ] **Step 7: Run all tests**

```bash
cargo test --workspace
```

Expected: green. The rename is mechanical and `latency` defaults to `None`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(kernel): rename ScorerResources to ResourceUsage, add latency

ResourceUsage will be used by both source-side (ProductionOutput,
Task 7+) and scorer-side (ScoreOutcome) accumulation. The latency
field anticipates per-call timing surfaced from the source envelope.

Per spec section 2.5.
EOF
)"
```

---

### Task 7: Add `ProductionOutput<O>` envelope type

**Files:**
- Modify: `evalkit/src/source.rs` (add `ProductionOutput`)
- Modify: `evalkit/src/lib.rs` (re-export)

- [ ] **Step 1: Write tests for the builder API**

Append to `evalkit/src/source.rs` test module:

```rust
#[test]
fn production_output_new_has_no_resources() {
    let p = ProductionOutput::new("answer".to_string());
    assert_eq!(p.output, "answer");
    assert!(p.usage.is_none());
    assert!(p.cost_usd.is_none());
    assert!(p.latency.is_none());
    assert!(p.metadata.is_empty());
}

#[test]
fn production_output_builder_sets_fields() {
    use std::time::Duration;
    use crate::TokenUsage;
    let usage = TokenUsage { input: 10, output: 20, cache_read: 0, cache_write: 0 };
    let p = ProductionOutput::new("x".to_string())
        .with_usage(usage.clone())
        .with_cost_usd(0.0125)
        .with_latency(Duration::from_millis(420))
        .with_metadata("model_id", serde_json::json!("claude-opus-4-7"));
    assert_eq!(p.usage, Some(usage));
    assert_eq!(p.cost_usd, Some(0.0125));
    assert_eq!(p.latency, Some(Duration::from_millis(420)));
    assert_eq!(p.metadata.get("model_id"), Some(&serde_json::json!("claude-opus-4-7")));
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib source::tests::production_output_new_has_no_resources
```

Expected: `ProductionOutput` not defined.

- [ ] **Step 3: Add the type**

In `evalkit/src/source.rs`:

```rust
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::TokenUsage;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProductionOutput<O> {
    pub output: O,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub latency: Option<Duration>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl<O> ProductionOutput<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
            usage: None,
            cost_usd: None,
            latency: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_cost_usd(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }

    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}
```

- [ ] **Step 4: Re-export**

`evalkit/src/lib.rs:110-112`:

```rust
pub use source::{
    OutputSource, OutputSourceError, ProductionOutput,
};
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p evalkit --lib source::tests
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/source.rs evalkit/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(kernel): add ProductionOutput<O> envelope type

The richer return shape for OutputSource::produce. Carries output plus
optional usage, cost, latency, and a freeform metadata bag.
Chat-completion-shaped fields (cache_hit, stop_reason, model_id) are
intentionally excluded — users put them in metadata; a domain crate
can ship a typed extension later.

Per spec section 2.1.
EOF
)"
```

---

### Task 8: Update `OutputSource` trait to return `ProductionOutput<O>`

**Files:**
- Modify: `evalkit/src/source.rs` (trait signature, blanket closure impl)
- Modify: `evalkit/src/task.rs` (Task::produce)
- Test: `evalkit/src/source.rs`

- [ ] **Step 1: Add a test that the trait now produces an envelope**

Append to `evalkit/src/source.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn output_source_returns_production_output() {
    struct EchoSource;
    impl OutputSource<String, String> for EchoSource {
        async fn produce(&self, input: &String) -> Result<ProductionOutput<String>, OutputSourceError> {
            Ok(ProductionOutput::new(input.clone()).with_cost_usd(0.001))
        }
    }
    let source = EchoSource;
    let result = source.produce(&"hi".to_string()).await.unwrap();
    assert_eq!(result.output, "hi");
    assert_eq!(result.cost_usd, Some(0.001));
}

#[tokio::test(flavor = "current_thread")]
async fn closure_blanket_impl_wraps_bare_output() {
    let source = |input: &String| {
        let input = input.clone();
        async move { Ok::<_, OutputSourceError>(input) }
    };
    let result = source.produce(&"hi".to_string()).await.unwrap();
    assert_eq!(result.output, "hi");
    assert!(result.usage.is_none());
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib source::tests::output_source_returns_production_output
```

Expected: trait signature mismatch (today's `produce` returns `Result<O, _>`).

- [ ] **Step 3: Update the trait**

`evalkit/src/source.rs`:

```rust
#[allow(async_fn_in_trait)]
pub trait OutputSource<I, O>: Send + Sync {
    async fn produce(&self, input: &I) -> Result<ProductionOutput<O>, OutputSourceError>;
    fn metadata_mode(&self) -> &'static str { "inline" }
}
```

Update the closure blanket impl to wrap bare output. The simple form (works for closures returning `Result<O, _>`):

```rust
impl<I, O, F, Fut> OutputSource<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<O, OutputSourceError>> + Send,
{
    async fn produce(&self, input: &I) -> Result<ProductionOutput<O>, OutputSourceError> {
        self(input).await.map(ProductionOutput::new)
    }
}
```

**Note on coherence:** providing a *second* blanket impl for closures that return `Result<ProductionOutput<O>, _>` would conflict with this one (Rust's coherence rules). Two ergonomic resolutions:

- **Option A (recommended for first cut):** keep one blanket impl that handles only bare-output closures. Closures wanting to return the envelope wrap themselves in a named type via `Task::from_fn` or a hand-written impl.
- **Option B (post-launch refinement):** use a marker trait or a sealed conversion to allow both. Defer until users complain.

Pick Option A for this task. Document the choice in `docs/decisions.md` as part of Task 9.

- [ ] **Step 4: Update `Task::produce`**

`evalkit/src/task.rs:88-100`:

```rust
impl<I, O> OutputSource<I, O> for Task<I, O>
where
    I: Send + Sync,
    O: Send + Sync,
{
    async fn produce(&self, input: &I) -> Result<ProductionOutput<O>, OutputSourceError> {
        (self.produce)(input).await.map(ProductionOutput::new)
    }

    fn metadata_mode(&self) -> &'static str {
        self.mode
    }
}
```

`Task` continues to wrap bare-output closures. A future `Task::from_envelope_fn` constructor can be added later for callers that want to surface usage; out of scope for this task.

- [ ] **Step 5: Run tests**

```bash
cargo test -p evalkit
```

Expected: kernel tests green. Other crates may not yet compile (they call `produce`); fix in Task 10.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/source.rs evalkit/src/task.rs
git commit -m "$(cat <<'EOF'
feat(kernel)!: OutputSource::produce returns ProductionOutput<O>

BREAKING: produce() return type changes from Result<O, _> to
Result<ProductionOutput<O>, _>. The closure blanket impl handles bare
Result<O, _> via ProductionOutput::new wrapping; closures wanting
explicit usage/cost/latency use a named impl or Task::from_fn.

Schema bump pending in Task 23.

Per spec section 2.1.
EOF
)"
```

---

### Task 9: Document the closure coherence decision

**Files:**
- Modify: `docs/decisions.md`

- [ ] **Step 1: Append the decision**

```bash
cat >> /home/laborant/evalkit.orig/docs/decisions.md <<'EOF'

## 2026-04-27 — Closure blanket impl: bare output only

For `OutputSource::produce(&self, &I) -> Result<ProductionOutput<O>, _>`,
we ship one blanket impl on closures: `Fn(&I) -> Future<Output = Result<O, _>>`.
The runtime wraps the bare result in `ProductionOutput::new`.

Rejected alternatives:
- A second blanket impl for closures returning `Result<ProductionOutput<O>, _>`
  conflicts with the bare-output impl by Rust's coherence rules.
- A marker-trait split adds complexity for a use case that's only marginal —
  closures wanting full envelope control already have `Task::from_fn` and can
  promote to a named type.

Revisit if a meaningful number of users hit this friction.
EOF
```

- [ ] **Step 2: Commit**

```bash
git add docs/decisions.md
git commit -m "docs: record closure coherence decision for OutputSource"
```

---

### Task 10: `Run::execute_trial` consumes `ProductionOutput`

**Files:**
- Modify: `evalkit/src/run.rs` (`execute_trial`, `produce_output_inner`, `ErasedOutputSource`)

- [ ] **Step 1: Update `ErasedOutputSource` to return the envelope**

`evalkit/src/run.rs:915-927`:

```rust
trait ErasedOutputSource<I, O>: Send + Sync {
    fn produce_boxed<'a>(
        &'a self,
        input: &'a I,
    ) -> Pin<Box<dyn Future<Output = Result<ProductionOutput<O>, OutputSourceError>> + 'a>>;
}

impl<I, O, S> ErasedOutputSource<I, O> for S
where
    S: OutputSource<I, O> + Send + Sync,
    O: 'static,
{
    fn produce_boxed<'a>(
        &'a self,
        input: &'a I,
    ) -> Pin<Box<dyn Future<Output = Result<ProductionOutput<O>, OutputSourceError>> + 'a>> {
        Box::pin(async move { self.produce(input).await })
    }
}
```

(Update the `OutputSourceFuture` type alias accordingly.)

- [ ] **Step 2: Update `produce_output_inner`**

`evalkit/src/run.rs:236-246`:

```rust
async fn produce_output_inner(
    &self,
    input: &I,
) -> Result<ProductionOutput<O>, OutputSourceError> {
    match self.sample_timeout {
        Some(duration) => {
            match timeout(duration, self.source.produce_boxed(input)).await {
                Ok(result) => result,
                Err(_) => Err(OutputSourceError::Timeout(duration)),
            }
        }
        None => self.source.produce_boxed(input).await,
    }
}
```

- [ ] **Step 3: Update `execute_trial` to read the envelope**

`evalkit/src/run.rs:174-226`. The `Ok(Ok(output))` arm changes:

```rust
Ok(Ok(production)) => {
    let ProductionOutput { output, usage, cost_usd, latency, metadata: source_metadata } = production;

    let mut source_resources = ResourceUsage::default();
    if let Some(usage) = usage {
        source_resources.token_usage = usage;
    }
    if let Some(cost) = cost_usd {
        source_resources.cost_usd = Some(cost);
    }
    if let Some(latency) = latency {
        source_resources.latency = Some(latency);
    }

    let ctx = ScorerContext {
        run_id,
        sample_id: &sample.id,
        trial_index,
        metadata: &sample.metadata,
        input: &sample.input,
        output: &output,
        reference: sample.reference.as_ref(),
    };

    match AssertUnwindSafe(self.executor.execute(&ctx)).catch_unwind().await {
        Ok(scores) => {
            let mut flattened = flatten_scores(scores);
            flattened.resources.merge(&source_resources);
            // Stash source_metadata on the trial result via the
            // FlattenedTrial — see Task 17 for the field addition.
            flattened.source_metadata = source_metadata;
            flattened
        }
        Err(_) => FlattenedTrial {
            scores: scorer_panic_scores(&self.definitions),
            resources: source_resources,
            source_metadata,
        },
    }
}
```

(`FlattenedTrial.source_metadata` doesn't exist yet — add it in Task 17. For now, leave the field assignment as a TODO comment in the code OR temporarily add the field with `HashMap::new()` default. The clean ordering is: add the field in Task 17, but if the build breaks now, add the field early as part of this task's commit.)

**Recommended:** add the `source_metadata: HashMap<String, Value>` field to `FlattenedTrial` here, and to `TrialResult` in Task 17. The plumbing is contiguous.

- [ ] **Step 4: Update `FlattenedTrial`**

In the same file (top of `run.rs`):

```rust
struct FlattenedTrial {
    scores: HashMap<String, Result<Score, ScorerError>>,
    resources: ResourceUsage,
    source_metadata: HashMap<String, Value>,
}
```

Update `flatten_scores`, `scorer_panic_scores` callers, and `source_failure_scores` callers to populate `source_metadata: HashMap::new()`.

- [ ] **Step 5: Run kernel tests**

```bash
cargo test -p evalkit
```

Expected: green. Provider crates may not yet compile (they implement the old trait); they fix in Phase 3.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/run.rs
git commit -m "$(cat <<'EOF'
feat(kernel): Run consumes ProductionOutput, accumulates source resources

execute_trial now reads usage/cost/latency from the envelope into a
source-side ResourceUsage and merges it with scorer-side resources.
source_metadata is stashed on FlattenedTrial pending Task 17 wiring it
into TrialResult.

Per spec section 2.1 / 2.5.
EOF
)"
```

---

### Task 11: Add `Budget` type to kernel

**Files:**
- Create: `evalkit/src/budget.rs`
- Modify: `evalkit/src/lib.rs`

- [ ] **Step 1: Write tests for `Budget` arithmetic**

`evalkit/src/budget.rs`:

```rust
use crate::ResourceUsage;

#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Budget {
    pub max_cost_usd: Option<f64>,
    pub max_tokens: Option<u64>,
}

impl Budget {
    pub fn max_cost_usd(mut self, max_cost_usd: f64) -> Self {
        self.max_cost_usd = Some(max_cost_usd);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Whether `additional` resource usage on top of nothing would exceed any
    /// configured cap. Advisory only — the kernel does not enforce.
    pub fn would_exceed(&self, additional: &ResourceUsage) -> bool {
        if let Some(max_cost) = self.max_cost_usd {
            if let Some(cost) = additional.cost_usd {
                if cost > max_cost {
                    return true;
                }
            }
        }
        if let Some(max_tokens) = self.max_tokens {
            let total =
                additional.token_usage.input + additional.token_usage.output;
            if total > max_tokens {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenUsage;

    #[test]
    fn would_exceed_cost_cap() {
        let budget = Budget::default().max_cost_usd(0.10);
        let usage = ResourceUsage::default().cost_usd(0.15);
        assert!(budget.would_exceed(&usage));
    }

    #[test]
    fn would_not_exceed_below_cap() {
        let budget = Budget::default().max_cost_usd(0.10);
        let usage = ResourceUsage::default().cost_usd(0.05);
        assert!(!budget.would_exceed(&usage));
    }

    #[test]
    fn would_exceed_token_cap() {
        let budget = Budget::default().max_tokens(100);
        let usage = ResourceUsage::default()
            .token_usage(TokenUsage { input: 60, output: 60, cache_read: 0, cache_write: 0 });
        assert!(budget.would_exceed(&usage));
    }

    #[test]
    fn no_caps_never_exceeds() {
        let budget = Budget::default();
        let usage = ResourceUsage::default().cost_usd(99.0);
        assert!(!budget.would_exceed(&usage));
    }
}
```

- [ ] **Step 2: Wire into lib.rs**

`evalkit/src/lib.rs`:

```rust
mod budget;
// ... existing modules ...

pub use budget::Budget;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p evalkit --lib budget::tests
```

Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add evalkit/src/budget.rs evalkit/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(kernel): add Budget type with cost and token caps

Advisory-only cost ceiling for scorers to consult via ScorerContext.
The runner does not enforce — that's a runtime middleware concern.

Per spec section 2.3.
EOF
)"
```

---

### Task 12: Add `tokio-util` dep, then `cancel` field on `ScorerContext`

**Files:**
- Modify: `evalkit/Cargo.toml`
- Modify: `evalkit/src/scorer_context.rs`

- [ ] **Step 1: Add `tokio-util` dep**

`evalkit/Cargo.toml`:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
futures = { version = "0.3", default-features = false, features = ["std", "async-await"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt", "time"] }
tokio-util = { version = "0.7", default-features = false }
uuid = { version = "1", features = ["serde", "v4"] }
```

- [ ] **Step 2: Write a test for `cancel` field default behavior**

Append to `evalkit/src/scorer_context.rs`:

```rust
#[test]
fn scorer_context_default_cancel_is_not_cancelled() {
    let input = String::from("p");
    let output = String::from("a");
    let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);
    assert!(!ctx.cancel.is_cancelled());
}
```

- [ ] **Step 3: Run, confirm fails**

```bash
cargo test -p evalkit --lib scorer_context::tests::scorer_context_default_cancel_is_not_cancelled
```

Expected: `cancel` field doesn't exist.

- [ ] **Step 4: Add the field with a static default**

`evalkit/src/scorer_context.rs`:

```rust
use std::collections::HashMap;
use std::sync::OnceLock;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

#[non_exhaustive]
pub struct ScorerContext<'a, I, O, R = ()> {
    pub run_id: &'a str,
    pub sample_id: &'a str,
    pub trial_index: usize,
    pub cancel: &'a CancellationToken,
    pub metadata: &'a HashMap<String, Value>,
    pub input: &'a I,
    pub output: &'a O,
    pub reference: Option<&'a R>,
}

fn default_cancel() -> &'static CancellationToken {
    static DEFAULT: OnceLock<CancellationToken> = OnceLock::new();
    DEFAULT.get_or_init(CancellationToken::new)
}

impl<'a, I, O, R> ScorerContext<'a, I, O, R> {
    pub fn new(input: &'a I, output: &'a O, reference: Option<&'a R>) -> Self {
        Self {
            run_id: "",
            sample_id: "",
            trial_index: 0,
            cancel: default_cancel(),
            metadata: empty_metadata(),
            input,
            output,
            reference,
        }
    }

    pub fn with_scope(
        run_id: &'a str,
        sample_id: &'a str,
        trial_index: usize,
        cancel: &'a CancellationToken,
        metadata: &'a HashMap<String, Value>,
        input: &'a I,
        output: &'a O,
        reference: Option<&'a R>,
    ) -> Self {
        Self {
            run_id,
            sample_id,
            trial_index,
            cancel,
            metadata,
            input,
            output,
            reference,
        }
    }
}
```

(`seed`, `budget`, `previous_scores` come in Task 13.)

- [ ] **Step 5: Update `Run::execute_trial` and the mapper executor to provide a `CancellationToken`**

In `evalkit/src/run.rs`, hold a `CancellationToken` on `Run`:

```rust
pub struct Run<I, O, R = ()> {
    // ... existing fields ...
    cancel: CancellationToken,
}
```

(Initialize in `build()` from `CancellationToken::new()`.)

In `execute_trial`, pass `&self.cancel` into the `ScorerContext::with_scope` call (or use the public field directly when constructing `ScorerContext`):

```rust
let ctx = ScorerContext {
    run_id,
    sample_id: &sample.id,
    trial_index,
    cancel: &self.cancel,
    metadata: &sample.metadata,
    input: &sample.input,
    output: &output,
    reference: sample.reference.as_ref(),
};
```

Same for the mapper executor's child `ScorerContext` constructions.

When the sample timeout fires, call `self.cancel.cancel()` for that trial's cancel scope. **Implementation note:** for cleanliness, give each trial its own child token via `cancel.child_token()` so trial-level cancellation doesn't tear down the whole run. Decide during implementation; document if you change this from per-run to per-trial.

- [ ] **Step 6: Run tests**

```bash
cargo test -p evalkit
```

Expected: green. Existing scorer impls inside the kernel use `ScorerContext::new` which provides the default token; nothing breaks.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(kernel): add CancellationToken to ScorerContext

Adds tokio-util as a dep. Scorers can check ctx.cancel.is_cancelled()
cooperatively. The runner holds a token on Run; scope decisions
(per-run vs per-trial) deferred to implementation.

Per spec section 2.3.
EOF
)"
```

---

### Task 13: Add `seed`, `budget`, `previous_scores` to `ScorerContext`

**Files:**
- Modify: `evalkit/src/scorer_context.rs`
- Modify: `evalkit/src/run.rs` (thread fields)
- Modify: `evalkit/src/score.rs` or `scorer_context.rs` (empty `previous_scores` static)

- [ ] **Step 1: Write tests for the new fields**

```rust
#[test]
fn scorer_context_carries_seed() {
    let input = String::from("p");
    let output = String::from("a");
    let metadata = HashMap::new();
    let cancel = CancellationToken::new();
    let previous = HashMap::new();
    let ctx: ScorerContext<'_, String, String> = ScorerContext::with_scope(
        "run-1", "s-1", 0,
        Some(42),
        &cancel,
        None,
        &previous,
        &metadata,
        &input,
        &output,
        None,
    );
    assert_eq!(ctx.seed, Some(42));
    assert!(ctx.budget.is_none());
    assert!(ctx.previous_scores.is_empty());
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib scorer_context::tests::scorer_context_carries_seed
```

Expected: `seed`, `budget`, `previous_scores` fields don't exist.

- [ ] **Step 3: Add the fields**

`evalkit/src/scorer_context.rs`:

```rust
use std::collections::HashMap;
use std::sync::OnceLock;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use crate::{Budget, Score};

#[non_exhaustive]
pub struct ScorerContext<'a, I, O, R = ()> {
    pub run_id: &'a str,
    pub sample_id: &'a str,
    pub trial_index: usize,
    pub seed: Option<u64>,
    pub cancel: &'a CancellationToken,
    pub budget: Option<&'a Budget>,
    pub previous_scores: &'a HashMap<String, Score>,
    pub metadata: &'a HashMap<String, Value>,
    pub input: &'a I,
    pub output: &'a O,
    pub reference: Option<&'a R>,
}

fn empty_previous_scores() -> &'static HashMap<String, Score> {
    static EMPTY: OnceLock<HashMap<String, Score>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

impl<'a, I, O, R> ScorerContext<'a, I, O, R> {
    pub fn new(input: &'a I, output: &'a O, reference: Option<&'a R>) -> Self {
        Self {
            run_id: "",
            sample_id: "",
            trial_index: 0,
            seed: None,
            cancel: default_cancel(),
            budget: None,
            previous_scores: empty_previous_scores(),
            metadata: empty_metadata(),
            input,
            output,
            reference,
        }
    }

    pub fn with_scope(
        run_id: &'a str,
        sample_id: &'a str,
        trial_index: usize,
        seed: Option<u64>,
        cancel: &'a CancellationToken,
        budget: Option<&'a Budget>,
        previous_scores: &'a HashMap<String, Score>,
        metadata: &'a HashMap<String, Value>,
        input: &'a I,
        output: &'a O,
        reference: Option<&'a R>,
    ) -> Self {
        Self {
            run_id, sample_id, trial_index, seed, cancel, budget,
            previous_scores, metadata, input, output, reference,
        }
    }
}
```

- [ ] **Step 4: Thread fields from `Run`**

In `evalkit/src/run.rs`:
- Add `budget: Option<Budget>` field to `Run` (and `RunBuilderWithTargets`). Add a builder method `.budget(b: Budget)`.
- In `execute_trial`, populate `seed`, `budget.as_ref()` and an empty initial `previous_scores` when constructing `ScorerContext`. Same for the mapper executor.
- The `previous_scores` map is grown by `ScorerSet` between scorers; for single-scorer `ScoringTarget` and across-target visibility, leave empty (per spec: visibility is per-set).

```rust
let previous_scores: HashMap<String, Score> = HashMap::new(); // populated by ScorerSet in Task 14
let ctx = ScorerContext {
    run_id,
    sample_id: &sample.id,
    trial_index,
    seed: self.seed,
    cancel: &self.cancel,
    budget: self.budget.as_ref(),
    previous_scores: &previous_scores,
    metadata: &sample.metadata,
    input: &sample.input,
    output: &output,
    reference: sample.reference.as_ref(),
};
```

- [ ] **Step 5: Run kernel tests**

```bash
cargo test -p evalkit
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/scorer_context.rs evalkit/src/run.rs
git commit -m "$(cat <<'EOF'
feat(kernel): add seed, budget, previous_scores to ScorerContext

seed: threaded from Run::seed for deterministic sampling.
budget: optional cost ceiling for advisory short-circuiting.
previous_scores: populated by ScorerSet (Task 14) within a set.

Per spec section 2.3.
EOF
)"
```

---

### Task 14: Wire `previous_scores` through `ScorerSet`

**Files:**
- Modify: `evalkit/src/scorer_set.rs`

- [ ] **Step 1: Write a test that a downstream scorer sees an upstream one**

In `evalkit/src/scorer_set.rs` test module:

```rust
#[tokio::test(flavor = "current_thread")]
async fn scorer_set_exposes_previous_scores() {
    use crate::{Score, ScoreDefinition, Scorer, ScorerContext, ScorerError, ScorerSet};

    struct First;
    impl Scorer<String, String, String> for First {
        async fn score(
            &self,
            _ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(true))
        }
        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("first")
        }
    }

    struct Second;
    impl Scorer<String, String, String> for Second {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            // sees "first" only if the set wired previous_scores correctly
            let saw_first = ctx.previous_scores.contains_key("first");
            Ok(Score::Binary(saw_first))
        }
        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("second")
        }
    }

    let set: ScorerSet<String, String, String> = ScorerSet::new()
        .scorer(First)
        .scorer(Second);

    let input = String::from("x");
    let output = String::from("x");
    let reference = String::from("");
    let ctx: ScorerContext<'_, String, String, String> =
        ScorerContext::new(&input, &output, Some(&reference));

    let results = set.score(&ctx).await;
    let second_result = results
        .iter()
        .find(|(d, _)| d.name == "second")
        .map(|(_, r)| r.as_ref().ok());
    assert!(matches!(
        second_result,
        Some(Some(outcome)) if matches!(outcome.score, Score::Binary(true))
    ));
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib scorer_set::tests::scorer_set_exposes_previous_scores
```

Expected: `Score::Binary(false)` because `previous_scores` isn't being threaded.

- [ ] **Step 3: Update `ScorerSet::score` to populate `previous_scores`**

`evalkit/src/scorer_set.rs`:

The set should execute scorers in declaration order and pass an *expanding* `previous_scores` map to each successive scorer. Pseudocode:

```rust
impl<I, O, R> ScorerSet<I, O, R> {
    pub async fn score<'a>(
        &'a self,
        ctx: &'a ScorerContext<'a, I, O, R>,
    ) -> Vec<(ScoreDefinition, Result<ScoreOutcome, ScorerError>)> {
        let mut results = Vec::with_capacity(self.scorers.len());
        let mut previous: HashMap<String, Score> = ctx.previous_scores.clone();

        for scorer in &self.scorers {
            let definition = scorer.definition();
            let inner_ctx = ScorerContext {
                run_id: ctx.run_id,
                sample_id: ctx.sample_id,
                trial_index: ctx.trial_index,
                seed: ctx.seed,
                cancel: ctx.cancel,
                budget: ctx.budget,
                previous_scores: &previous,
                metadata: ctx.metadata,
                input: ctx.input,
                output: ctx.output,
                reference: ctx.reference,
            };

            let outcome_result = scorer.score_with_resources(&inner_ctx).await;
            if let Ok(outcome) = &outcome_result {
                previous.insert(definition.name.clone(), outcome.score.clone());
            }
            results.push((definition, outcome_result));
        }

        results
    }
}
```

(Adapt to the actual `ScorerSet` shape — internal state may use trait objects.)

- [ ] **Step 4: Run tests**

```bash
cargo test -p evalkit
```

Expected: green, including the new `scorer_set_exposes_previous_scores` test.

- [ ] **Step 5: Commit**

```bash
git add evalkit/src/scorer_set.rs
git commit -m "$(cat <<'EOF'
feat(kernel): ScorerSet exposes previous scores to downstream scorers

Within a set, scorers execute in declaration order; each sees the
successful results of all earlier scorers via ctx.previous_scores.
Failed scorers do not appear in the map.

Cross-set visibility is intentionally omitted — scorer sets remain
independent units.

Per spec section 2.3.
EOF
)"
```

---

### Task 15: Extend `ScoreOutcome` with `reasoning` and `metadata`

**Files:**
- Modify: `evalkit/src/scorer.rs`

- [ ] **Step 1: Test the new fields**

Append to `evalkit/src/scorer.rs` test module:

```rust
#[test]
fn score_outcome_carries_reasoning_and_metadata() {
    use serde_json::json;
    let outcome = ScoreOutcome::new(Score::Binary(true))
        .with_reasoning("matches the gold")
        .with_metadata("rubric", json!({ "version": 1 }));
    assert_eq!(outcome.reasoning.as_deref(), Some("matches the gold"));
    assert_eq!(outcome.metadata.get("rubric"), Some(&json!({ "version": 1 })));
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib scorer::tests::score_outcome_carries_reasoning_and_metadata
```

Expected: `with_reasoning`/`with_metadata` not defined.

- [ ] **Step 3: Extend the type**

`evalkit/src/scorer.rs`:

```rust
use std::collections::HashMap;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScoreOutcome {
    pub score: Score,
    pub resources: ResourceUsage,
    pub reasoning: Option<String>,
    pub metadata: HashMap<String, Value>,
}

impl ScoreOutcome {
    pub fn new(score: Score) -> Self {
        Self {
            score,
            resources: ResourceUsage::default(),
            reasoning: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_resources(mut self, resources: ResourceUsage) -> Self {
        self.resources = resources;
        self
    }

    pub fn with_reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p evalkit
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add evalkit/src/scorer.rs
git commit -m "$(cat <<'EOF'
feat(kernel): ScoreOutcome carries reasoning and metadata

These move out of Score::Structured (Task 19/20) and live next to the
score on the per-call outcome instead of inside a score variant. Lets
Binary, Numeric, Label scorers all carry rationale.

Per spec section 2.2.
EOF
)"
```

---

### Task 16: Add `ScoredEntry` to `TrialResult`

**Files:**
- Modify: `evalkit/src/run_result.rs`
- Modify: `evalkit/src/run.rs` (`flatten_scores`, `source_failure_scores`)
- Modify: `evalkit/src/stats.rs` (consume new shape)
- Modify: `evalkit/src/comparison.rs` (consume new shape)

- [ ] **Step 1: Write a test for `ScoredEntry` round-trip**

Append to `evalkit/src/run_result.rs`:

```rust
#[cfg(test)]
mod entry_tests {
    use super::*;

    #[test]
    fn scored_entry_serializes_and_deserializes() {
        use serde_json::json;
        let entry = ScoredEntry {
            result: Ok(Score::Binary(true)),
            reasoning: Some("matches".to_string()),
            metadata: HashMap::from([("k".to_string(), json!("v"))]),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let round_trip: ScoredEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.reasoning.as_deref(), Some("matches"));
    }
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib run_result::entry_tests::scored_entry_serializes_and_deserializes
```

Expected: `ScoredEntry` not defined.

- [ ] **Step 3: Add `ScoredEntry`, change `TrialResult.scores`**

`evalkit/src/run_result.rs`:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ScoredEntry {
    #[serde(with = "score_result_serde")]
    pub result: Result<Score, ScorerError>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrialResult {
    pub scores: HashMap<String, ScoredEntry>,
    pub duration: Duration,
    pub trial_index: usize,
    #[serde(default)]
    pub source_metadata: HashMap<String, Value>,
}
```

(Replace the existing `score_results_serde` module to operate on a single `Result<Score, ScorerError>` per entry rather than a map. The existing module's serialize/deserialize functions become helpers used by `ScoredEntry`'s `#[serde(with = "score_result_serde")]`.)

```rust
mod score_result_serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    enum ScoreResultOwned {
        Ok(Score),
        Err(String),
    }

    pub fn serialize<S>(
        result: &Result<Score, ScorerError>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match result {
            Ok(score) => ScoreResultOwned::Ok(score.clone()),
            Err(err) => ScoreResultOwned::Err(err.to_string()),
        };
        value.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Result<Score, ScorerError>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = ScoreResultOwned::deserialize(deserializer)?;
        Ok(match raw {
            ScoreResultOwned::Ok(score) => Ok(score),
            ScoreResultOwned::Err(message) => {
                Err(ScorerError::internal(SerializedScorerError(message)))
            }
        })
    }

    #[derive(Debug)]
    struct SerializedScorerError(String);

    impl std::fmt::Display for SerializedScorerError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for SerializedScorerError {}
}
```

- [ ] **Step 4: Update `flatten_scores`, `scorer_panic_scores`, `source_failure_scores` in `run.rs`**

Replace `HashMap<String, Result<Score, ScorerError>>` with `HashMap<String, ScoredEntry>` everywhere those functions touch. Each entry is constructed with `reasoning` and `metadata` from the `ScoreOutcome`:

```rust
fn flatten_scores(results: TrialScores) -> FlattenedTrial {
    let mut scores = HashMap::with_capacity(results.len());
    let mut resources = ResourceUsage::default();

    for (definition, result) in results {
        let entry = match result {
            Ok(outcome) => {
                resources.merge(&outcome.resources);
                ScoredEntry {
                    result: validate_score(outcome.score),
                    reasoning: outcome.reasoning,
                    metadata: outcome.metadata,
                }
            }
            Err(err) => ScoredEntry {
                result: Err(err),
                reasoning: None,
                metadata: HashMap::new(),
            },
        };
        scores.insert(definition.name, entry);
    }

    FlattenedTrial { scores, resources, source_metadata: HashMap::new() }
}
```

Similarly for the synthetic error helpers — they produce `ScoredEntry { result: Err(...), reasoning: None, metadata: HashMap::new() }`.

- [ ] **Step 5: Update `RunStats` and `Comparison` to read `entry.result`**

`evalkit/src/stats.rs:58-72`:

```rust
for (scorer_name, entry) in &trial.scores {
    match &entry.result {
        Ok(score) => {
            accumulators
                .entry(scorer_name.clone())
                .and_modify(|accumulator| accumulator.add_score(&sample.sample_id, score))
                .or_insert_with(|| ScorerAccumulator::from_score(&sample.sample_id, score));
        }
        Err(_) => total_errors += 1,
    }
}
```

`evalkit/src/comparison.rs`: similar — anywhere it reads `trial.scores[name]` as `Result<Score, _>`, switch to `entry.result`.

- [ ] **Step 6: Run all tests**

```bash
cargo test -p evalkit
```

Expected: green.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(kernel)!: TrialResult.scores becomes HashMap<String, ScoredEntry>

ScoredEntry { result, reasoning, metadata } replaces the bare
Result<Score, ScorerError>. Reasoning lives next to the score, not
inside a Score variant. source_metadata field added to TrialResult.

Schema-breaking; bump pending in Task 23.

Per spec section 2.2.
EOF
)"
```

---

### Task 17: Wire `source_metadata` from `Run` into `TrialResult`

**Files:**
- Modify: `evalkit/src/run.rs` (`execute_trial` populates `TrialResult.source_metadata`)

- [ ] **Step 1: Test that source metadata reaches TrialResult**

Append to `evalkit/src/run.rs` tests:

```rust
#[tokio::test(flavor = "current_thread")]
async fn source_metadata_reaches_trial_result() {
    use serde_json::json;
    let dataset = Dataset::new(vec![
        Sample::builder("x".to_string()).id("s1").build().unwrap(),
    ]);

    struct EnvelopeSource;
    impl OutputSource<String, String> for EnvelopeSource {
        async fn produce(&self, input: &String) -> Result<ProductionOutput<String>, OutputSourceError> {
            Ok(ProductionOutput::new(input.clone())
                .with_metadata("model_id", json!("test-model")))
        }
    }
    struct EchoScorer;
    impl Scorer<String, String> for EchoScorer {
        async fn score(&self, _ctx: &ScorerContext<'_, String, String>) -> Result<Score, ScorerError> {
            Ok(Score::Binary(true))
        }
        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("echo")
        }
    }

    let run = Run::builder()
        .dataset(dataset)
        .source(EnvelopeSource)
        .scorer(EchoScorer)
        .build()
        .unwrap();
    let result = run.execute().await.unwrap();
    let trial = &result.samples[0].trials[0];
    assert_eq!(trial.source_metadata.get("model_id"), Some(&json!("test-model")));
}
```

- [ ] **Step 2: Run, confirm passes if Task 10/16 already wired it; fix otherwise**

```bash
cargo test -p evalkit --lib run::tests::source_metadata_reaches_trial_result
```

Expected: passes (Tasks 10 and 16 should have wired this through `FlattenedTrial.source_metadata` → `TrialResult`).

If it fails: locate where `FlattenedTrial` is converted to `TrialResult` in `execute_trial` (around line 218-225) and ensure `source_metadata` is copied into `TrialResult`:

```rust
ExecutedTrial {
    result: TrialResult {
        scores: flattened.scores,
        duration: started.elapsed(),
        trial_index,
        source_metadata: flattened.source_metadata,
    },
    resources: flattened.resources,
}
```

- [ ] **Step 3: Commit (if any fix needed)**

```bash
git add evalkit/src/run.rs
git commit -m "feat(kernel): TrialResult.source_metadata populated from ProductionOutput"
```

(No commit if Task 10/16 already covered this; the test is a regression guard.)

---

### Task 18: `SampleResult` carries `source_resources` and `scorer_resources`

**Files:**
- Modify: `evalkit/src/run_result.rs`
- Modify: `evalkit/src/run.rs` (`execute_sample` populates the split fields)

- [ ] **Step 1: Test the new fields**

Append to `evalkit/src/run_result.rs`:

```rust
#[cfg(test)]
mod sample_result_tests {
    use super::*;

    #[test]
    fn sample_result_default_resources_are_zero() {
        let sr = SampleResult {
            sample_id: "s1".to_string(),
            trials: vec![],
            trial_count: 0,
            scored_count: 0,
            error_count: 0,
            token_usage: TokenUsage::default(),
            cost_usd: None,
            source_resources: ResourceUsage::default(),
            scorer_resources: ResourceUsage::default(),
        };
        assert_eq!(sr.source_resources, ResourceUsage::default());
    }
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib run_result::sample_result_tests
```

Expected: `source_resources` and `scorer_resources` fields missing.

- [ ] **Step 3: Add the fields**

`evalkit/src/run_result.rs`:

```rust
use crate::ResourceUsage;

#[derive(Debug, Serialize, Deserialize)]
pub struct SampleResult {
    pub sample_id: String,
    pub trials: Vec<TrialResult>,
    pub trial_count: usize,
    pub scored_count: usize,
    pub error_count: usize,
    #[serde(default)]
    pub token_usage: TokenUsage,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub source_resources: ResourceUsage,
    #[serde(default)]
    pub scorer_resources: ResourceUsage,
}
```

(`token_usage` and `cost_usd` are kept as the union for back-compat read paths; they're computed as `source_resources + scorer_resources`.)

- [ ] **Step 4: Update `execute_sample` to track the split**

`evalkit/src/run.rs:148-172`:

```rust
async fn execute_sample(&self, run_id: &str, sample: &Sample<I, R>) -> SampleResult {
    let mut trials = Vec::with_capacity(self.trial_count);
    let mut scorer_resources = ResourceUsage::default();
    let mut source_resources = ResourceUsage::default();

    for trial_index in 0..self.trial_count {
        let trial = self.execute_trial(run_id, sample, trial_index).await;
        // execute_trial returns ExecutedTrial { result, scorer_resources, source_resources }
        scorer_resources.merge(&trial.scorer_resources);
        source_resources.merge(&trial.source_resources);
        trials.push(trial.result);
    }

    let scored_count = trials
        .iter()
        .filter(|trial| trial.scores.values().any(|e| e.result.is_ok()))
        .count();

    let mut combined = source_resources.clone();
    combined.merge(&scorer_resources);

    SampleResult {
        sample_id: sample.id.clone(),
        trial_count: self.trial_count,
        error_count: self.trial_count - scored_count,
        scored_count,
        trials,
        token_usage: combined.token_usage,
        cost_usd: combined.cost_usd,
        source_resources,
        scorer_resources,
    }
}
```

This requires `ExecutedTrial` to carry both resource bundles separately. Update accordingly (currently `ExecutedTrial` has a single `resources` field):

```rust
struct ExecutedTrial {
    result: TrialResult,
    scorer_resources: ResourceUsage,
    source_resources: ResourceUsage,
}
```

In `execute_trial`'s `Ok(Ok(production))` arm, populate both:

```rust
ExecutedTrial {
    result: TrialResult {
        scores: flattened.scores,
        duration: started.elapsed(),
        trial_index,
        source_metadata: flattened.source_metadata,
    },
    scorer_resources: flattened.resources,
    source_resources,
}
```

(`source_resources` is the local destructured from `ProductionOutput`.)

- [ ] **Step 5: Run tests**

```bash
cargo test -p evalkit
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add evalkit/src/run.rs evalkit/src/run_result.rs
git commit -m "$(cat <<'EOF'
feat(kernel)!: SampleResult splits source_resources and scorer_resources

The unified token_usage / cost_usd fields remain (back-compat read
path) but the per-side breakdown is now first-class. Cost dashboards
can attribute spend to source vs judge.

Per spec section 2.5.
EOF
)"
```

---

### Task 19: Migrate in-tree `Score::Structured` users to `ScoreOutcome::reasoning`

**Files:**
- Modify: `evalkit-scorers-llm/src/lib.rs:562, 614, 681, 1012, 1042, 1560, 1799, 1844, 1938`
- Modify: `evalkit-scorers-llm/src/lib.rs:1220` (the error message about label judges)
- Modify: `evalkit/src/comparison.rs:150, 156`
- Modify: `evalkit/src/run.rs:1305`
- Modify: `evalkit/src/scorer_ext.rs:482`
- Modify: `evalkit/src/stats.rs:185, 200`

- [ ] **Step 1: Migrate llm-judge constructors**

For each `Score::Structured { score, reasoning, metadata }` constructor in `evalkit-scorers-llm/src/lib.rs`, transform:

```rust
// before
ScoreOutcome::new(Score::Structured {
    score: numeric_value,
    reasoning: rationale,
    metadata: extras,
})

// after
let mut outcome = ScoreOutcome::new(Score::Numeric(numeric_value))
    .with_reasoning(rationale);
for (k, v) in extras_map {
    outcome = outcome.with_metadata(k, v);
}
outcome
```

For binary judges:

```rust
// before
ScoreOutcome::new(Score::Structured { score: if pass { 1.0 } else { 0.0 }, reasoning, metadata })

// after
ScoreOutcome::new(Score::Binary(pass)).with_reasoning(reasoning)
```

The error message at line 1220 changes — label judges *can now* capture reasoning. Remove the constraint and the error variant if appropriate.

- [ ] **Step 2: Update kernel `Score::Structured` consumers**

`evalkit/src/comparison.rs:150, 156`: drop the `Score::Structured` arms (the variant goes away in Task 20).

`evalkit/src/scorer_ext.rs:482`: drop the `Score::Structured` arm in the weighted-score extractor (`Some(score)` was the rationale-bundled path; weighted scorers now route through `Score::Numeric` only).

`evalkit/src/run.rs:1305`: drop the `Score::Structured` validation arm. The `validate_score` function's structured branch is removed; `Score::Numeric` already handles finite checks.

`evalkit/src/stats.rs:185, 200`: drop the `Score::Structured` accumulation paths; `Numeric` covers the use case.

- [ ] **Step 3: Run all tests**

```bash
cargo test --workspace
```

Expected: green. Tests in `evalkit-scorers-llm` that asserted on `Score::Structured` need updating to look at `ScoreOutcome.reasoning` and the base score variant.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor: migrate Score::Structured users to ScoreOutcome::reasoning

LlmJudge, g_eval, llm_classifier, calibrated_llm_classifier all now
return base Score variants (Numeric/Binary/Label) with reasoning
attached via ScoreOutcome. Label judges newly support reasoning
(previously rejected because Structured required a numeric score).

Comparison, scorer_ext, run::validate_score, and stats accumulator
arms for Structured are removed; the variant itself goes away in
Task 20.

Per spec section 2.2.
EOF
)"
```

---

### Task 20: Remove `Score::Structured` variant

**Files:**
- Modify: `evalkit/src/score.rs`

- [ ] **Step 1: Test that the variant is gone**

Append to `evalkit/src/score.rs`:

```rust
#[cfg(test)]
mod variant_tests {
    use super::*;

    #[test]
    fn score_has_only_clean_variants() {
        // Compile-time assertion via exhaustive match; this fails to compile
        // if anyone adds a new variant. Intentional regression guard.
        fn _exhaust(s: Score) {
            match s {
                Score::Numeric(_) => {}
                Score::Binary(_) => {}
                Score::Label(_) => {}
                Score::Metric { .. } => {}
            }
        }
    }
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib score::variant_tests
```

Expected: compile error — `Score::Structured` still exists.

- [ ] **Step 3: Drop the variant**

`evalkit/src/score.rs`:

```rust
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Score {
    Numeric(f64),
    Binary(bool),
    Label(String),
    Metric { name: String, value: f64, unit: Option<String> },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ScoreSerde {
    Numeric { value: f64 },
    Binary { value: bool },
    Label { value: String },
    Metric { name: String, value: f64, unit: Option<String> },
}

impl Serialize for Score {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let value = match self {
            Self::Numeric(value) => ScoreSerde::Numeric { value: *value },
            Self::Binary(value) => ScoreSerde::Binary { value: *value },
            Self::Label(value) => ScoreSerde::Label { value: value.clone() },
            Self::Metric { name, value, unit } => ScoreSerde::Metric {
                name: name.clone(), value: *value, unit: unit.clone(),
            },
        };
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Score {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Ok(match ScoreSerde::deserialize(deserializer)? {
            ScoreSerde::Numeric { value } => Self::Numeric(value),
            ScoreSerde::Binary { value } => Self::Binary(value),
            ScoreSerde::Label { value } => Self::Label(value),
            ScoreSerde::Metric { name, value, unit } => Self::Metric { name, value, unit },
        })
    }
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test --workspace
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add evalkit/src/score.rs
git commit -m "$(cat <<'EOF'
feat(kernel)!: remove Score::Structured variant

All callers migrated to base Score variants in Task 19. Reasoning
lives on ScoreOutcome; structured numeric breakdowns live in
ScoreOutcome::metadata or as separate Score::Metric entries.

Per spec section 2.2.
EOF
)"
```

---

### Task 21: Update `RunStats` accumulator (clean up post-Structured)

**Files:**
- Modify: `evalkit/src/stats.rs`

- [ ] **Step 1: Verify the stats accumulator no longer references `Score::Structured`**

```bash
grep -n "Structured" /home/laborant/evalkit.orig/evalkit/src/stats.rs
```

Expected: zero matches (Task 19 removed them).

- [ ] **Step 2: Confirm tests pass**

```bash
cargo test -p evalkit --lib stats
```

Expected: green.

- [ ] **Step 3: No commit needed** unless the grep showed stragglers.

---

### Task 22: `RunStats.mixed_variant_scorers` surfaces dropped scorers

**Files:**
- Modify: `evalkit/src/stats.rs`

- [ ] **Step 1: Test the surfacing**

Append to `evalkit/src/stats.rs` test module:

```rust
#[test]
fn mixed_variant_scorers_are_surfaced() {
    use crate::{Sample, ScoredEntry, TrialResult, RunMetadata};
    use std::collections::HashMap;
    use chrono::Utc;
    use std::time::Duration;

    let metadata = RunMetadata {
        run_id: "r1".to_string(), seed: None,
        dataset_fingerprint: String::new(), scorer_fingerprint: String::new(),
        code_commit: None, code_fingerprint: None,
        judge_model_pins: vec![],
        started_at: Utc::now(), completed_at: Utc::now(),
        duration: Duration::ZERO, trial_count: 1,
        score_definitions: vec![], source_mode: "inline".to_string(),
    };

    // Sample with two trials emitting different Score variants for the same scorer
    let trials = vec![
        TrialResult {
            scores: HashMap::from([("flaky".to_string(), ScoredEntry {
                result: Ok(Score::Numeric(0.5)),
                reasoning: None, metadata: HashMap::new(),
            })]),
            duration: Duration::ZERO, trial_index: 0, source_metadata: HashMap::new(),
        },
        TrialResult {
            scores: HashMap::from([("flaky".to_string(), ScoredEntry {
                result: Ok(Score::Binary(true)),
                reasoning: None, metadata: HashMap::new(),
            })]),
            duration: Duration::ZERO, trial_index: 1, source_metadata: HashMap::new(),
        },
    ];

    let result = RunResult {
        metadata,
        samples: vec![SampleResult {
            sample_id: "s1".to_string(), trials, trial_count: 2,
            scored_count: 2, error_count: 0,
            token_usage: TokenUsage::default(), cost_usd: None,
            source_resources: ResourceUsage::default(),
            scorer_resources: ResourceUsage::default(),
        }],
    };

    let stats = result.stats();
    assert!(stats.mixed_variant_scorers.contains(&"flaky".to_string()));
    assert!(!stats.scorer_stats.contains_key("flaky"));
}
```

- [ ] **Step 2: Run, confirm fails**

```bash
cargo test -p evalkit --lib stats::tests::mixed_variant_scorers_are_surfaced
```

Expected: `mixed_variant_scorers` field doesn't exist.

- [ ] **Step 3: Add the field and route Mixed cases into it**

`evalkit/src/stats.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunStats {
    pub scorer_stats: HashMap<String, ScorerStats>,
    #[serde(default)]
    pub mixed_variant_scorers: Vec<String>,
    pub total_samples: usize,
    pub trials_per_sample: usize,
    pub total_trials_executed: usize,
    pub total_errors: usize,
}
```

In `RunResult::stats_with`:

```rust
let (scorer_stats, mixed_variant_scorers) = {
    let mut clean = HashMap::new();
    let mut mixed = Vec::new();
    for (name, accumulator) in accumulators {
        match accumulator {
            ScorerAccumulator::Mixed => mixed.push(name),
            other => {
                if let Some(stats) = other.finish(confidence_level) {
                    clean.insert(name, stats);
                }
            }
        }
    }
    mixed.sort();
    (clean, mixed)
};

RunStats {
    scorer_stats,
    mixed_variant_scorers,
    total_samples: self.samples.len(),
    trials_per_sample: self.metadata.trial_count,
    total_trials_executed: self.samples.len() * self.metadata.trial_count,
    total_errors,
}
```

Update `RunStats::summary` to print the warning line:

```rust
if !self.mixed_variant_scorers.is_empty() {
    lines.push(format!(
        "mixed-variant scorers (dropped from stats): {}",
        self.mixed_variant_scorers.join(", ")
    ));
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p evalkit --lib stats
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add evalkit/src/stats.rs
git commit -m "$(cat <<'EOF'
feat(kernel): RunStats surfaces mixed-variant scorers

Previously, scorers whose trials emit different Score variants were
silently dropped from RunStats.scorer_stats. They now appear in
RunStats.mixed_variant_scorers and are mentioned in summary().

Per spec section 2.4.
EOF
)"
```

---

### Task 23: Bump `RUN_RESULT_SCHEMA_VERSION`

**Files:**
- Modify: `evalkit/src/schema.rs`

- [ ] **Step 1: Bump the constant**

```rust
// evalkit/src/schema.rs
pub const RUN_RESULT_SCHEMA_VERSION: &str = "3";
```

- [ ] **Step 2: Run jsonl tests**

```bash
cargo test -p evalkit --lib jsonl
```

Expected: green. (`jsonl.rs` tests use the constant indirectly; if any test hard-codes "2", update it.)

- [ ] **Step 3: Commit**

```bash
git add evalkit/src/schema.rs
git commit -m "$(cat <<'EOF'
chore(kernel)!: bump RUN_RESULT_SCHEMA_VERSION 2 -> 3

Reflects the bundled Phase 2 schema redesign (ProductionOutput,
ScoredEntry, source_resources/scorer_resources split). Old readers
fail loud per the policy in docs/stability.md.
EOF
)"
```

---

### Task 24: Verify JSONL schema header enforcement

**Files:**
- Verify: `evalkit/src/jsonl.rs`
- Test: `evalkit/src/jsonl.rs`

- [ ] **Step 1: Confirm the writer emits the header**

```bash
grep -n "schema_version" /home/laborant/evalkit.orig/evalkit/src/jsonl.rs
```

Expected: `Header { schema_version: RUN_RESULT_SCHEMA_VERSION }` written first; reader checks `record_schema_version != RUN_RESULT_SCHEMA_VERSION`.

If both present: nothing to do.

- [ ] **Step 2: Add an explicit fail-loud test**

```rust
#[test]
fn read_jsonl_rejects_old_schema() {
    use std::io::Cursor;
    let old_jsonl = r#"{"Header":{"schema_version":"2"}}
{"some":"data"}
"#;
    let result = read_jsonl::<RunResult, _>(Cursor::new(old_jsonl));
    assert!(result.is_err(), "must reject old schema");
}
```

(Adjust to the actual `read_jsonl` API.)

- [ ] **Step 3: Run**

```bash
cargo test -p evalkit --lib jsonl
```

Expected: green.

- [ ] **Step 4: Commit (if test was new)**

```bash
git add evalkit/src/jsonl.rs
git commit -m "test(kernel): assert read_jsonl rejects old schema versions"
```

---

### Task 25: `evalkit migrate-runlog` CLI subcommand

**Files:**
- Modify: `evalkit-cli/src/main.rs` (or wherever subcommands are dispatched)
- Create: `evalkit-cli/src/migrate.rs`

- [ ] **Step 1: Add subcommand registration**

In the CLI's clap setup, add:

```rust
.subcommand(
    clap::Command::new("migrate-runlog")
        .about("Migrate a run-log JSONL file from schema v2 to v3")
        .arg(clap::Arg::new("in").long("in").required(true))
        .arg(clap::Arg::new("out").long("out").required(true)),
)
```

- [ ] **Step 2: Implement the v2 → v3 transform**

`evalkit-cli/src/migrate.rs`:

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use serde_json::Value;

pub fn migrate_v2_to_v3(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);
    let mut writer = BufWriter::new(File::create(output)?);

    // Header: rewrite schema_version to "3"
    let mut first_line = true;
    for line in std::io::BufRead::lines(reader) {
        let line = line?;
        if line.is_empty() { continue; }
        let mut value: Value = serde_json::from_str(&line)?;

        if first_line {
            // Header line — bump version
            if let Some(header) = value.get_mut("Header") {
                header["schema_version"] = Value::String("3".into());
            }
            first_line = false;
        } else {
            // RunResult body — transform Score::Structured into base+reasoning
            transform_run_result(&mut value);
        }

        writeln!(writer, "{}", serde_json::to_string(&value)?)?;
    }
    writer.flush()?;
    Ok(())
}

fn transform_run_result(value: &mut Value) {
    let Some(samples) = value.get_mut("samples").and_then(|v| v.as_array_mut()) else { return; };
    for sample in samples {
        let Some(trials) = sample.get_mut("trials").and_then(|v| v.as_array_mut()) else { continue; };
        for trial in trials {
            let Some(scores) = trial.get_mut("scores").and_then(|v| v.as_object_mut()) else { continue; };
            for (_name, entry) in scores.iter_mut() {
                transform_score_entry(entry);
            }
            // Old schema didn't have source_metadata; default empty
            if !trial.as_object().unwrap().contains_key("source_metadata") {
                trial["source_metadata"] = Value::Object(Default::default());
            }
        }
        // Old schema didn't have source_resources / scorer_resources; default empty
        let sample_obj = sample.as_object_mut().unwrap();
        sample_obj.entry("source_resources").or_insert_with(|| serde_json::json!({
            "token_usage": {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0},
            "cost_usd": null, "latency": null
        }));
        sample_obj.entry("scorer_resources").or_insert_with(|| serde_json::json!({
            "token_usage": {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0},
            "cost_usd": null, "latency": null
        }));
    }
}

fn transform_score_entry(entry: &mut Value) {
    // Old: "Ok": { "type": "structured", "score": <f>, "reasoning": <s>, "metadata": <obj> }
    // New: result: Ok(Numeric(<f>)), reasoning: <s>, metadata: <obj>
    if let Some(ok) = entry.pointer("/Ok").cloned() {
        if let Some("structured") = ok.get("type").and_then(|t| t.as_str()) {
            let score = ok.get("score").cloned().unwrap_or(Value::Null);
            let reasoning = ok.get("reasoning").cloned().unwrap_or(Value::Null);
            let metadata = ok.get("metadata").cloned().unwrap_or(Value::Object(Default::default()));
            *entry = serde_json::json!({
                "result": { "Ok": { "type": "numeric", "value": score } },
                "reasoning": reasoning,
                "metadata": metadata,
            });
            return;
        }
    }
    // Otherwise: old shape was Result<Score,_> directly; wrap into ScoredEntry
    let old_result = entry.clone();
    *entry = serde_json::json!({
        "result": old_result,
        "reasoning": null,
        "metadata": {},
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_score_migrates_to_numeric_with_reasoning() {
        let mut entry = serde_json::json!({
            "Ok": { "type": "structured", "score": 0.75, "reasoning": "good", "metadata": {"k": "v"} }
        });
        transform_score_entry(&mut entry);
        assert_eq!(entry["result"]["Ok"]["type"], "numeric");
        assert_eq!(entry["result"]["Ok"]["value"], 0.75);
        assert_eq!(entry["reasoning"], "good");
        assert_eq!(entry["metadata"]["k"], "v");
    }

    #[test]
    fn binary_score_migrates_with_null_reasoning() {
        let mut entry = serde_json::json!({
            "Ok": { "type": "binary", "value": true }
        });
        transform_score_entry(&mut entry);
        assert_eq!(entry["result"]["Ok"]["type"], "binary");
        assert_eq!(entry["reasoning"], Value::Null);
    }
}
```

- [ ] **Step 3: Wire it into the CLI dispatch**

```rust
match matches.subcommand() {
    Some(("migrate-runlog", m)) => {
        let input = std::path::PathBuf::from(m.get_one::<String>("in").unwrap());
        let output = std::path::PathBuf::from(m.get_one::<String>("out").unwrap());
        evalkit_cli::migrate::migrate_v2_to_v3(&input, &output)?;
        println!("Migrated {} -> {}", input.display(), output.display());
    }
    // ... other subcommands ...
}
```

- [ ] **Step 4: Run CLI tests**

```bash
cargo test -p evalkit-cli
```

Expected: green, including the new migrate tests.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(cli): add migrate-runlog subcommand for v2 -> v3 JSONL

One-shot transform: rewrites schema_version header, splits
Score::Structured into Numeric + reasoning, fills default empty
source_metadata / source_resources / scorer_resources.
EOF
)"
```

---

**Phase 2 review checkpoint.** At this point:

- Kernel ships `ProductionOutput<O>` envelope; `OutputSource::produce` returns it.
- `ScorerContext` has `seed`, `cancel`, `budget`, `previous_scores`.
- `TrialResult.scores` uses `ScoredEntry`; reasoning lives there.
- `Score::Structured` is gone.
- `RunStats` surfaces mixed-variant scorers.
- `SampleResult` splits source/scorer resources.
- Schema bumped to `"3"`.
- `evalkit migrate-runlog` available.

Workspace builds; downstream crates may have compile errors until Phase 3.

---

## Phase 3 — downstream propagation

### Task 26: Update `evalkit-providers` to envelope return

**Files:**
- Modify: `evalkit-providers/src/lib.rs` (`HttpAcquisition::produce`, `SubprocessAcquisition::produce`)

- [ ] **Step 1: Update each `OutputSource` impl to return `ProductionOutput`**

For each impl, change the return type and wrap the bare result. Where the underlying response carries usage / cost (HTTP plugin response, subprocess JSON response), parse those fields and attach via `with_usage`/`with_cost_usd`/`with_latency`.

Example for `HttpAcquisition`:

```rust
impl OutputSource<String, String> for HttpAcquisition {
    async fn produce(&self, input: &String) -> Result<ProductionOutput<String>, OutputSourceError> {
        let started = Instant::now();
        let response_json = self.post(input).await?;
        let output = response_json
            .get(&self.output_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| OutputSourceError::ExecutionFailed(/* ... */))?
            .to_string();

        let mut envelope = ProductionOutput::new(output)
            .with_latency(started.elapsed());

        if let Some(usage_obj) = response_json.get("usage") {
            // optional convention: HTTP plugin may return {"usage": {"input": N, "output": N, ...}}
            if let Ok(usage) = serde_json::from_value::<TokenUsage>(usage_obj.clone()) {
                envelope = envelope.with_usage(usage);
            }
        }
        if let Some(cost) = response_json.get("cost_usd").and_then(|v| v.as_f64()) {
            envelope = envelope.with_cost_usd(cost);
        }

        Ok(envelope)
    }
}
```

`SubprocessAcquisition` follows the same pattern; the JSON response convention should be documented in `docs/plugin-protocol.md` as part of the v2 protocol bump (out of scope for this plan; note in TODOs).

- [ ] **Step 2: Run provider tests**

```bash
cargo test -p evalkit-providers
```

Expected: green. Update fixtures to the envelope shape.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(providers)!: HTTP/Subprocess sources return ProductionOutput

Surface usage, cost, and latency from plugin responses when present.
The plugin-protocol doc bump for declaring the usage/cost JSON
convention is tracked separately.
EOF
)"
```

---

### Task 27: Update `evalkit-scorers-llm` for new `ScoreOutcome`

**Files:**
- Modify: `evalkit-scorers-llm/src/lib.rs` (already partially done in Task 19)

- [ ] **Step 1: Verify all `Score::Structured` constructions are gone**

```bash
grep -n "Score::Structured" /home/laborant/evalkit.orig/evalkit-scorers-llm/src/lib.rs
```

Expected: zero.

- [ ] **Step 2: Confirm tests compile and pass**

```bash
cargo test -p evalkit-scorers-llm
```

Expected: green. Any remaining failures are due to test fixtures asserting on old shapes; update each.

- [ ] **Step 3: If g_eval / llm_classifier expose typed metadata helpers, ensure they wire through to `ScoreOutcome::with_metadata` rather than into a struct's `metadata` field**

Spot-check:

```bash
grep -n "with_metadata\|with_reasoning" /home/laborant/evalkit.orig/evalkit-scorers-llm/src/lib.rs | head
```

Should show extensive use.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(scorers-llm): finalize ScoreOutcome migration

All Score::Structured constructions are now ScoreOutcome::with_reasoning
+ base variant. Label judges newly support reasoning per spec 2.2.
EOF
)"
```

---

### Task 28: Update remaining scorer crates and exporters

**Files:**
- Modify: `evalkit-scorers-text/src/lib.rs`
- Modify: `evalkit-scorers-rag/src/lib.rs`
- Modify: `evalkit-scorers-embed/src/lib.rs`
- Modify: `evalkit-scorers-redteam/src/lib.rs`
- Modify: `evalkit-exporters-langfuse/src/lib.rs`

- [ ] **Step 1: Run workspace tests; address per-crate compile errors**

```bash
cargo test --workspace
```

The non-llm scorer crates likely don't construct `Score::Structured` directly; the main change is the `ScorerResources` → `ResourceUsage` rename, which Task 6 already propagated. Check for stragglers:

```bash
grep -rn "ScorerResources" /home/laborant/evalkit.orig/ --include="*.rs"
```

- [ ] **Step 2: Update Langfuse exporter for `ScoredEntry`**

`evalkit-exporters-langfuse/src/lib.rs`: anywhere it reads `trial.scores[name]` as `Result<Score, _>`, switch to `entry.result`. Reasoning can be exported as a Langfuse trace observation; metadata likewise.

- [ ] **Step 3: Run**

```bash
cargo test --workspace
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(workspace): propagate kernel 2.0 type changes to scorers and exporters

ScorerResources -> ResourceUsage rename, ScoredEntry shape on TrialResult.
Langfuse exporter forwards reasoning + metadata to traces.
EOF
)"
```

---

### Task 29: `evalkit-server` SQLite migration

**Files:**
- Modify: `evalkit-server/src/storage.rs` (or wherever `RunStore` lives)
- Add: `evalkit-server/migrations/` SQL or rusqlite migration code

- [ ] **Step 1: Identify the schema version field on stored runs**

```bash
grep -n "schema_version\|SCHEMA_VERSION" /home/laborant/evalkit.orig/evalkit-server/src/*.rs
```

The server stores `RunResult` blobs (likely as JSON). The migration is to walk each row, run the same `transform_run_result` from Task 25, and bump the stored version tag.

- [ ] **Step 2: Add a migration function**

```rust
pub fn migrate_storage_v2_to_v3(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT id, payload FROM stored_runs WHERE schema_version = '2'")?;
    let rows: Vec<(i64, String)> = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    for (id, payload) in rows {
        let mut value: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        evalkit_cli::migrate::transform_run_result(&mut value);
        let new_payload = serde_json::to_string(&value)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE stored_runs SET payload = ?, schema_version = '3' WHERE id = ?",
            rusqlite::params![new_payload, id],
        )?;
    }
    Ok(())
}
```

(The `transform_run_result` function should be made `pub` in `evalkit-cli/src/migrate.rs` so the server can reuse it; alternatively, lift it into a small `evalkit-migrate` crate. Decide during implementation.)

- [ ] **Step 3: Add a smoke test**

In `evalkit-server` tests, populate a temporary SQLite with one v2-shaped run, run the migration, assert the row now has `schema_version = '3'` and the payload's structured-score has been transformed.

- [ ] **Step 4: Run**

```bash
cargo test -p evalkit-server
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(server): SQLite migration v2 -> v3 for stored runs

Reuses the transform from evalkit-cli::migrate to walk each stored
RunResult JSON blob and rewrite it to the new schema. Bumps
schema_version column to '3' on success.
EOF
)"
```

---

### Task 30: Final integration sweep — workspace builds clean, all tests green

- [ ] **Step 1: Workspace build**

```bash
cargo build --workspace --all-targets
```

Expected: zero errors, zero warnings.

- [ ] **Step 2: Workspace tests**

```bash
cargo test --workspace
```

Expected: all green.

- [ ] **Step 3: Crate version bumps**

Bump every workspace crate from 0.x or 1.0 to `2.0.0` in `Cargo.toml` files. Update `Cargo.lock`.

```bash
grep -rn "^version" /home/laborant/evalkit.orig/*/Cargo.toml
```

For each, change to `version = "2.0.0"`.

- [ ] **Step 4: Update `docs/decisions.md` with the 2.0 release record**

Add an entry documenting:
- The output envelope shape decision
- The reasoning-out-of-Score decision (rejected `Reasoned(Box<Score>, String)` alternative)
- The `previous_scores` per-set visibility scope
- The closure coherence trade-off (already added in Task 9)

- [ ] **Step 5: Update `CHANGELOG.md` (or create one if missing)**

Document all breaking changes for users:
- `OutputSource::produce` returns `ProductionOutput<O>`
- `Score::Structured` removed
- `ScorerResources` renamed to `ResourceUsage`
- `TrialResult.scores` uses `ScoredEntry`
- `SampleResult` adds `source_resources` / `scorer_resources`
- `ScorerContext` gains `seed`, `cancel`, `budget`, `previous_scores`
- `OutputSnapshot`/`SourceOutput` moved to `evalkit-runtime`
- OTel-specific concerns moved to `evalkit-otel`

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore: release 2.0.0

Bundle the kernel output API redesign into a single 2.0 release.
See docs/superpowers/specs/2026-04-27-kernel-output-api-2.0-design.md
and CHANGELOG.md.
EOF
)"
```

- [ ] **Step 7: Tag**

```bash
git tag -a v2.0.0 -m "evalkit 2.0 — kernel output API redesign"
```

(Don't push the tag automatically; the user decides when to publish.)

---

## Self-Review Notes

After writing this plan, reviewed against the spec and applied these fixes inline:

1. **Spec coverage:** Mapped every spec section to a task — 1.1→Task 5, 1.2→Task 4, 1.3→Task 2, 1.4→Task 3, 1.5→Task 1, 2.1→Tasks 7/8/9/10, 2.2→Tasks 15/16/19/20, 2.3→Tasks 11/12/13/14, 2.4→Task 22, 2.5→Tasks 6/18. Migration & schema → Tasks 23/24/25. Downstream → Tasks 26-30.
2. **Type consistency:** `ScorerResources` → `ResourceUsage` rename happens in Task 6 before any later task uses the new name. `ScoreOutcome` extension (Task 15) precedes its use in Task 16's `ScoredEntry`.
3. **Placeholder scan:** Replaced spec-style `...` ellipses in code blocks with explicit field listings. Implementation-detail decisions (closure coherence, mapper executor unsafe vs identity, OtelObserver task-local replacement, `transform_run_result` location) are flagged as "decide during implementation; document in `docs/decisions.md`" — these are real open questions, not placeholders.
4. **Phase boundaries are review checkpoints**, called out explicitly between Phase 1 / Phase 2 / Phase 3.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-27-kernel-output-api-2.0.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using `executing-plans`, batch execution with checkpoints.

Which approach?
