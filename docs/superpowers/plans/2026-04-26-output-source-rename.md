# OutputSource Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the `Acquisition` umbrella to `OutputSource` across the workspace, collapse the facade to a single `.source(...)` method, add a `Task<I, O>` adapter, rename `evalkit-otel::Observe` → `OtelObserver`, and bump all wire formats — as a clean 1.0 break with no compat shim.

**Architecture:** One layered API: facade `Eval::new(s).source(thing)`, kernel `Run::builder().source(thing)`, trait `OutputSource<I, O>` with `produce(...)`. Active uses come through `Task::from_fn(...)`/`Task::http(...)`/`Task::subprocess(...)` or via the closure blanket impl; passive uses come through concrete types in adapter crates (`OtelObserver` and future fixture/log/metric sources). No passive umbrella struct; the trait is the umbrella.

**Tech Stack:** Rust workspace (cargo, tokio, serde, async-trait via `async fn` in trait); Python plugin SDK (`python/evalkit_plugin`); TypeScript plugin SDK (`typescript/evalkit_plugin`); SQLite via `evalkit-server`; JSONL run logs.

**Spec reference:** `docs/superpowers/specs/2026-04-26-output-source-naming-design.md`.

---

## Symbol rename quick-reference

| Old | New |
|---|---|
| `Acquisition` (trait) | `OutputSource` |
| `acquire` (method) | `produce` |
| `acquire_with_snapshots` | `produce_with_snapshots` |
| `AcquisitionError` | `OutputSourceError` |
| `AcquisitionMetadata` | `SourceMetadata` |
| `AcquisitionSnapshot<O>` | `OutputSnapshot<O>` |
| `AcquiredOutput<O>` | `SourceOutput<O>` |
| `ErasedAcquisition<I, O>` (trait, run.rs internal) | `ErasedOutputSource<I, O>` |
| `AcquisitionFuture<'a, O>` (run.rs internal) | `OutputSourceFuture<'a, O>` |
| `acquire_boxed` | `produce_boxed` |
| `acquire_output` (Run internal) | `produce_output` |
| `acquire_output_inner` | `produce_output_inner` |
| `NoAcquisition` (RunBuildError variant) | `NoSource` |
| `acquisition_mode` (struct field, JSON field, SQL column) | `source_mode` |
| Module `evalkit/src/acquisition.rs` | `evalkit/src/source.rs` |
| `Eval::acquire(...)` | `Eval::source(...)` |
| `Run::builder().acquisition(...)` | `Run::builder().source(...)` |
| Plugin protocol `kind: "acquisition"` | `kind: "source"` |
| TOML `[acquisition]` | `[source]` |
| JSONL schema `run-log-v1.schema.json` | `run-log-v2.schema.json` |
| `RUN_RESULT_SCHEMA_VERSION = "1"` | `"2"` |
| `evalkit-otel::Observe` | `evalkit-otel::OtelObserver` |
| `python/evalkit_plugin/examples/echo_acquisition.py` | `echo_source.py` |
| `typescript/evalkit_plugin/examples/echo_acquisition.ts` | `echo_source.ts` |

---

## Task 1: Pre-flight baseline

**Files:** none modified.

- [ ] **Step 1: Read the spec end-to-end.**

Read `docs/superpowers/specs/2026-04-26-output-source-naming-design.md`. Confirm familiarity with the layered API, the rename table, and the "no compat shim" sequencing.

- [ ] **Step 2: Verify clean baseline — workspace compiles.**

Run: `cargo check --workspace --all-targets`
Expected: clean build, no warnings about the symbols being renamed (a few unrelated warnings are acceptable).

- [ ] **Step 3: Verify clean baseline — workspace tests pass.**

Run: `cargo test --workspace`
Expected: all tests pass. If anything fails on `main`, stop and investigate before starting renames — fixing existing breakage is out of scope for this plan.

- [ ] **Step 4: Verify doctests pass.**

Run: `cargo test --workspace --doc`
Expected: all doctests pass. The boundary-contract doctests in `evalkit/src/lib.rs` (the `compile_fail` blocks) must still hold after the rename.

- [ ] **Step 5: Confirm no in-progress changes that would conflict.**

Run: `git status`
Expected: working tree clean. If there are unrelated unstaged changes, stash them before proceeding.

---

## Task 2: Workspace-wide Rust symbol rename (lockstep)

This task renames every Rust symbol in the rename table simultaneously across the workspace. The workspace will not compile mid-task; it must compile at task end. One commit at the end. Many steps (one per file or per-crate group), so the implementer can pause and verify between steps.

**Files (modify):**
- `evalkit/src/acquisition.rs` → renamed to `evalkit/src/source.rs`
- `evalkit/src/lib.rs`
- `evalkit/src/run.rs`
- `evalkit/src/run_result.rs`
- `evalkit/src/eval.rs`
- `evalkit/src/schema.rs`
- `evalkit/tests/boundary_kernel_surface.rs`
- `evalkit/tests/comparison.rs`
- `evalkit/tests/jsonl.rs`
- `evalkit/tests/run.rs`
- `evalkit/tests/run_result.rs`
- `evalkit/tests/stats.rs`
- `evalkit/examples/basic.rs`
- `evalkit/examples/quickstart.rs`
- `evalkit/examples/prod_eval_daemon.rs`
- `evalkit-runtime/src/lib.rs`
- `evalkit-runtime/examples/throughput_bench.rs`
- `evalkit-otel/src/lib.rs`
- `evalkit-otel/tests/observe.rs`
- `evalkit-providers/src/lib.rs`
- `evalkit-providers/tests/python_shims.rs`
- `evalkit-server/src/lib.rs`
- `evalkit-exporters-langfuse/src/lib.rs`
- `evalkit-exporters-langfuse/tests/export_run.rs`
- `evalkit-cli/src/main.rs`
- `evalkit-cli/tests/diff.rs`

**Files (rename via `git mv`):**
- `evalkit/src/acquisition.rs` → `evalkit/src/source.rs`

- [ ] **Step 1: Rename the module file.**

```bash
git mv evalkit/src/acquisition.rs evalkit/src/source.rs
```

- [ ] **Step 2: Update `evalkit/src/lib.rs` — module declaration and re-exports.**

In `evalkit/src/lib.rs`:
- Replace `pub mod acquisition;` with `pub mod source;`
- In the `pub use acquisition::{...}` block, change to `pub use source::{...}` and update the imported names: `AcquiredOutput, Acquisition, AcquisitionError, AcquisitionMetadata, AcquisitionSnapshot` → `SourceOutput, OutputSource, OutputSourceError, SourceMetadata, OutputSnapshot`.
- In the `pub mod prelude { pub use crate::{...} }` block, update the same identifiers in the re-export list.

- [ ] **Step 3: Rewrite `evalkit/src/source.rs` with new symbols.**

Apply these renames inside the file:
- `pub struct AcquisitionMetadata` → `pub struct SourceMetadata`
- `pub struct AcquisitionSnapshot<O>` → `pub struct OutputSnapshot<O>`
- `pub struct AcquiredOutput<O>` → `pub struct SourceOutput<O>`
- `pub enum AcquisitionError` → `pub enum OutputSourceError`
- `pub trait Acquisition<I, O>` → `pub trait OutputSource<I, O>`
- `async fn acquire(...)` → `async fn produce(...)` (trait method)
- `async fn acquire_with_snapshots(...)` → `async fn produce_with_snapshots(...)` (trait method, default impl)
- All internal references inside the file (impl blocks, blanket impl, error messages, tests) updated accordingly.
- Inline error message strings: replace "acquisition" → "output source" where appropriate (`"acquisition execution failed"` → `"output source execution failed"`, `"acquisition timed out"` → `"output source timed out"`, `"acquisition panicked"` → `"output source panicked"`).
- The `task_local! { static CURRENT_SAMPLE_ID: String; }` block, `current_sample_id()`, and `with_current_sample_id()` are unchanged.
- The blanket `impl<I, O, F, Fut> OutputSource<I, O> for F where F: Fn(&I) -> Fut + ...` keeps the same shape; only the trait name and method name update.
- The test module renames: `PrefixAcquisition` struct → `PrefixSource`; `acquisition_trait_supports_custom_implementations` → `output_source_trait_supports_custom_implementations`; `acquisition_blanket_impl_supports_async_closures` → `output_source_blanket_impl_supports_async_closures`; `acquisition_error_*` test names → `output_source_error_*`.
- Add a module-level rustdoc comment at the top of `source.rs` summarizing the kernel role per the spec's "Choosing a Source" guidance: `//! Output source abstraction. //! //! `OutputSource` is the kernel umbrella for "produce evaluation output for a sample." //! Most evals use `Task::from_fn` or a closure (active). To evaluate an already-instrumented //! system, use a passive source from an adapter crate (e.g., `evalkit-otel::OtelObserver`). //!`

- [ ] **Step 4: Update `evalkit/src/run.rs` — trait usage, internal helpers, field renames, builder method.**

Apply these renames:
- Imports (top of file): `Acquisition, AcquisitionError` → `OutputSource, OutputSourceError`.
- `RunBuildError::NoAcquisition` → `RunBuildError::NoSource`. Update the `Display` impl error message: `"run is missing an acquisition"` → `"run is missing an output source"`.
- All struct fields named `acquisition: Box<dyn ErasedAcquisition<I, O>>` → `source: Box<dyn ErasedOutputSource<I, O>>`.
- All struct fields named `acquisition_mode: &'static str` (and `acquisition_mode: String`) → `source_mode`.
- Method `async fn acquire_output(...)` → `async fn produce_output(...)`.
- Method `async fn acquire_output_inner(...)` → `async fn produce_output_inner(...)`.
- Generic bound `A: Acquisition<I, O>` → `S: OutputSource<I, O>`.
- Method `pub fn acquisition<O, A>(self, acquisition: A) -> RunBuilderConfigured<I, O, R>` → `pub fn source<O, S>(self, source: S) -> RunBuilderConfigured<I, O, R>`.
- Local binding `let acquisition_mode = acquisition.metadata().mode;` → `let source_mode = source.metadata().mode;`.
- All `acquisition_mode: self.acquisition_mode` initializers → `source_mode: self.source_mode`.
- All `acquisition_mode: this.acquisition_mode` initializers → `source_mode: this.source_mode`.
- The `if self.acquisition_mode == "observe"` branch → `if self.source_mode == "observe"` (the value `"observe"` stays the same — that's a runtime tag, not a type name).
- Trait `trait ErasedAcquisition<I, O>: Send + Sync { fn acquire_boxed(...) -> AcquisitionFuture<'a, O>; }` → `trait ErasedOutputSource<I, O>: Send + Sync { fn produce_boxed(...) -> OutputSourceFuture<'a, O>; }`.
- The blanket impl `impl<I, O, A> ErasedAcquisition<I, O> for A where A: Acquisition<I, O> + Send + Sync` → `impl<I, O, S> ErasedOutputSource<I, O> for S where S: OutputSource<I, O> + Send + Sync`. Inside, `self.acquire(input)` → `self.produce(input)`.
- Type alias `type AcquisitionFuture<'a, O>` → `type OutputSourceFuture<'a, O>`.
- Helper function `acquisition_failure_scores(...)` → `source_failure_scores(...)`.

- [ ] **Step 5: Update `evalkit/src/run_result.rs` — struct field rename.**

`pub acquisition_mode: String,` → `pub source_mode: String,`. Update any tests in this file that construct `RunMetadata` literally.

- [ ] **Step 6: Update `evalkit/src/eval.rs` — facade method rename.**

- Imports: `Acquisition` → `OutputSource`.
- Doc comment example showing `.acquire(acquisition)` → `.source(source)`.
- `pub fn acquire<O, A>(self, acquisition: A) -> EvalTask<I, O, R> where A: Acquisition<I, O> + 'static` → `pub fn source<O, S>(self, source: S) -> EvalTask<I, O, R> where S: OutputSource<I, O> + 'static`.
- Inside, `Run::builder().dataset(self.dataset).acquisition(acquisition)` → `Run::builder().dataset(self.dataset).source(source)`.
- Update `EvalTask` doc comment if it mentions "acquisition."
- In the test module: `AcquisitionError` import → `OutputSourceError`. Closure type annotations `Ok::<_, AcquisitionError>(...)` → `Ok::<_, OutputSourceError>(...)`. The test uses `.acquire(facade_acquire)` → `.source(facade_acquire)`. The kernel-path comparison uses `.acquisition(kernel_acquire)` → `.source(kernel_acquire)`. Local variable names `facade_acquire`/`kernel_acquire` may stay (developer choice) or rename to `facade_source`/`kernel_source` for consistency — pick rename for consistency.
- Assertion `facade_result.metadata.acquisition_mode == kernel_result.metadata.acquisition_mode` → `source_mode`.

- [ ] **Step 7: Update `evalkit/src/schema.rs` — bump version constant and schema file path.**

```rust
pub const RUN_RESULT_SCHEMA_VERSION: &str = "1";
```
→
```rust
pub const RUN_RESULT_SCHEMA_VERSION: &str = "2";
```

And:
```rust
serde_json::from_str(include_str!("../../docs/schema/run-log-v1.schema.json"))
```
→
```rust
serde_json::from_str(include_str!("../../docs/schema/run-log-v2.schema.json"))
```

(The schema file itself is renamed in Task 4; this step assumes the file will exist by the time the workspace compiles. If `cargo check` complains about a missing file at the end of Task 2, do a one-step rename of the schema file path here to prevent the workspace from being stuck.)

- [ ] **Step 8: Update tests in `evalkit/tests/`.**

For each of the 6 test files (`boundary_kernel_surface.rs`, `comparison.rs`, `jsonl.rs`, `run.rs`, `run_result.rs`, `stats.rs`): apply the symbol rename table to imports, type annotations, builder calls, and field accesses. The pattern is:
- `use evalkit::{Acquisition, AcquisitionError, AcquiredOutput, AcquisitionSnapshot, AcquisitionMetadata, ...}` → renamed.
- Calls to `.acquisition(...)` on the run builder → `.source(...)`.
- Calls to `.acquire(...)` on `Eval` → `.source(...)`.
- Field accesses `.acquisition_mode` → `.source_mode`.
- Trait impls `impl Acquisition<...> for MyType` → `impl OutputSource<...> for MyType`. Trait method `async fn acquire(...)` → `async fn produce(...)`. Method `acquire_with_snapshots` → `produce_with_snapshots`.

- [ ] **Step 9: Update examples in `evalkit/examples/`.**

For `basic.rs`, `quickstart.rs`, `prod_eval_daemon.rs`: same pattern as Step 8. Update imports, builder calls, and any prose comments (e.g., `// acquisition that returns ...` → `// source that returns ...`).

- [ ] **Step 10: Update `evalkit-runtime/src/lib.rs`.**

The runtime executor consumes `Acquisition`. Apply the rename: imports, generic bounds, trait usage. Same pattern. The runtime crate is internal (semver-unstable per the boundary contract), so churn here is fine.

- [ ] **Step 11: Update `evalkit-runtime/examples/throughput_bench.rs`.**

Apply the rename to the benchmark example.

- [ ] **Step 12: Update `evalkit-otel/src/lib.rs` — `Observe` → `OtelObserver` and rename all consumers.**

- `pub struct Observe` → `pub struct OtelObserver`.
- `impl Observe { pub fn new(...) -> Self ... }` → `impl OtelObserver { pub fn new(...) -> Self ... }`.
- `impl<I> Acquisition<I, Vec<Span>> for Observe` → `impl<I> OutputSource<I, Vec<Span>> for OtelObserver`.
- The `acquire` method body inside the impl → `produce` method body.
- The `metadata()` method returning `AcquisitionMetadata { mode: "observe" }` → `SourceMetadata { mode: "observe" }`.
- Any inline doc comments referencing "Observe" → "OtelObserver".
- Imports: `use evalkit::{Acquisition, AcquisitionError, AcquisitionMetadata}` → `use evalkit::{OutputSource, OutputSourceError, SourceMetadata}`.
- Add or update the crate-level rustdoc at the top of `evalkit-otel/src/lib.rs` to identify `OtelObserver` as the passive `OutputSource` exported by this crate, per the spec's pedagogical-compensation note (one or two sentences is enough).

- [ ] **Step 13: Update `evalkit-otel/tests/observe.rs`.**

Apply the rename to imports, test setup, and assertions. Test function names that include "observe" describing the *concept* (not the type name) can stay; only update where the type name `Observe` appears in code.

- [ ] **Step 14: Update `evalkit-providers/src/lib.rs`.**

The HTTP and subprocess plugins implement `Acquisition`. Apply the rename: imports, trait impls (`impl Acquisition<...> for HttpAcquisition` → `impl OutputSource<...> for HttpAcquisition`; same for subprocess), method renames, error variants.

Note: the type names `HttpAcquisition` and `SubprocessAcquisition` (or whatever exists in the file) **do not** rename in this step. They are concrete adapter types and will be folded into the `Task<I, O>` umbrella in Task 3. For now, just update their `impl Acquisition` → `impl OutputSource`.

- [ ] **Step 15: Update `evalkit-providers/tests/python_shims.rs`.**

Apply the rename pattern.

- [ ] **Step 16: Update `evalkit-server/src/lib.rs` — Rust struct + SQL schema + queries.**

- Rust struct field: `pub acquisition_mode: String` (line 72) → `pub source_mode: String`.
- SQL `SELECT run_id, started_at, completed_at, acquisition_mode, sample_count FROM runs ...` → `... source_mode ...`.
- `row.get(3)?` mapping into `acquisition_mode` field → into `source_mode`.
- SQL `INSERT OR REPLACE INTO runs (run_id, started_at, completed_at, acquisition_mode, sample_count, ...)` → `... source_mode ...`.
- Bind value `run.result.metadata.acquisition_mode` → `run.result.metadata.source_mode`.
- `CREATE TABLE` schema (line 651): `acquisition_mode TEXT NOT NULL` → `source_mode TEXT NOT NULL`.
- HTML rendering: `mode = escape_html(&run.acquisition_mode)` → `mode = escape_html(&run.source_mode)`. Same for the other `escape_html` call.
- Test fixture (line 2257): `acquisition_mode: String::from("inline")` → `source_mode: String::from("inline")`.

- [ ] **Step 17: Update `evalkit-exporters-langfuse/src/lib.rs` and tests.**

Apply the rename to any `acquisition_mode` field accesses or trait references.

- [ ] **Step 18: Update `evalkit-cli/src/main.rs` and `evalkit-cli/tests/diff.rs`.**

Apply the rename. The CLI parses the TOML config; the in-Rust enum/struct names that map TOML keys will rename in Task 5. For Step 18, only update Rust symbol references that overlap with the kernel rename (e.g., `Acquisition` import, `acquisition_mode` field). TOML key parsing stays referring to `[acquisition]` for now — that wire format change is Task 5.

- [ ] **Step 19: Run `cargo check --workspace --all-targets`.**

Expected: clean compile. If anything fails, fix before moving on. Common issues at this point: a missed `acquire` call, a missed import alias, a missed field access. Use `cargo check` errors as a checklist.

- [ ] **Step 20: Run `cargo test --workspace`.**

Expected: all tests pass. The schema-version test in `evalkit/tests/run_result.rs` will now expect `"2"` instead of `"1"` — update that assertion to match the new constant. The run-log schema doctest will fail until the schema JSON file is renamed in Task 4; if that's the only failing test, accept it and proceed.

- [ ] **Step 21: Commit.**

```bash
git add -A evalkit/ evalkit-runtime/ evalkit-otel/ evalkit-providers/ evalkit-server/ evalkit-exporters-langfuse/ evalkit-cli/
git commit -m "refactor: rename Acquisition to OutputSource workspace-wide

Renames the kernel umbrella trait and all associated types, methods,
errors, internal helpers, and consuming code across the workspace.
Includes the Eval/Run builder method rename to .source(), the runtime
field rename acquisition_mode -> source_mode, and the Observe ->
OtelObserver rename in evalkit-otel.

Wire formats (plugin protocol JSON, TOML config, JSONL schema file,
SQLite migrations beyond the Rust struct) are updated in follow-up
commits in this series."
```

---

## Task 3: Add `Task<I, O>` adapter type

The active umbrella. Wraps closures, HTTP plugins, and subprocess plugins behind one named type so users can pass `Task::http(...)` or `Task::subprocess(...)` to `.source(...)` instead of constructing the underlying type directly.

**Files:**
- Create: `evalkit/src/task.rs`
- Modify: `evalkit/src/lib.rs`
- Modify: `evalkit-providers/src/lib.rs` (expose constructors used by `Task::http` and `Task::subprocess`)
- Test: `evalkit/src/task.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test for `Task::from_fn`.**

In `evalkit/src/task.rs` (new file), at the bottom in a `#[cfg(test)] mod tests { ... }` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputSource;

    #[tokio::test(flavor = "current_thread")]
    async fn task_from_fn_implements_output_source() {
        let task: Task<String, String> = Task::from_fn(|input: &String| {
            let input = input.clone();
            async move { Ok(format!("echo::{input}")) }
        });

        let input = String::from("hello");
        let output = task.produce(&input).await.unwrap();
        assert_eq!(output, "echo::hello");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn task_from_fn_metadata_reports_inline_mode() {
        let task: Task<String, String> = Task::from_fn(|_input: &String| async move {
            Ok(String::from("ok"))
        });
        assert_eq!(task.metadata().mode, "inline");
    }
}
```

- [ ] **Step 2: Run tests; verify they fail because `Task` doesn't exist.**

Run: `cargo test -p evalkit task_from_fn_implements_output_source`
Expected: FAIL (`unresolved import` or `cannot find type Task`).

- [ ] **Step 3: Implement minimal `Task<I, O>` with `from_fn`.**

In `evalkit/src/task.rs`:

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::source::{OutputSource, OutputSourceError, SourceMetadata};

type ProduceFn<I, O> = Arc<
    dyn Fn(&I) -> Pin<Box<dyn Future<Output = Result<O, OutputSourceError>> + Send>>
        + Send
        + Sync,
>;

pub struct Task<I, O> {
    produce: ProduceFn<I, O>,
    mode: &'static str,
}

impl<I, O> Task<I, O>
where
    I: 'static,
    O: 'static,
{
    pub fn from_fn<F, Fut>(f: F) -> Self
    where
        F: Fn(&I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, OutputSourceError>> + Send + 'static,
    {
        let f = Arc::new(f);
        Self {
            produce: Arc::new(move |input: &I| {
                let f = Arc::clone(&f);
                Box::pin(async move { f(input).await })
            }),
            mode: "inline",
        }
    }
}

impl<I, O> OutputSource<I, O> for Task<I, O>
where
    I: Send + Sync,
    O: Send + Sync,
{
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError> {
        (self.produce)(input).await
    }

    fn metadata(&self) -> SourceMetadata {
        SourceMetadata { mode: self.mode }
    }
}
```

Note: the closure signature in `from_fn` borrows `&I` for the duration of the call; the boxed-future variant requires the future to own its data. If borrowing causes lifetime errors during Step 4, switch the boxed signature to take `I: Clone` and clone before await — but try the borrowed form first since the existing `Acquisition` blanket impl does the same.

- [ ] **Step 4: Add `pub mod task;` and re-export to `evalkit/src/lib.rs`.**

In `evalkit/src/lib.rs`:
- Add `mod task;` (private module).
- In the `pub use` block: `pub use task::Task;`.
- In `pub mod prelude { pub use crate::{...} }`: add `Task` to the list.

- [ ] **Step 5: Run tests; verify pass.**

Run: `cargo test -p evalkit task_from_fn_implements_output_source task_from_fn_metadata_reports_inline_mode`
Expected: PASS.

- [ ] **Step 6: Write failing test for `Task::http`.**

First, look at `evalkit-providers/src/lib.rs` to find the public constructor for the HTTP plugin type. Look for something like `HttpAcquisition::new(...)` or `HttpAcquisition::with_url(...)`. Use the simplest constructor that takes a URL or config struct; no live HTTP call happens in this test because `metadata()` does not invoke `produce`.

```rust
#[tokio::test(flavor = "current_thread")]
async fn task_http_metadata_reports_http_mode() {
    // Concrete constructor — adapt to whatever signature evalkit-providers exposes.
    // Example: HttpAcquisition::new("http://localhost:0".parse().unwrap())
    let plugin = evalkit_providers::HttpAcquisition::new(
        "http://localhost:0".parse().expect("valid url"),
    );
    let task = Task::<serde_json::Value, serde_json::Value>::http(plugin);
    assert_eq!(task.metadata().mode, "http");
}
```

If `HttpAcquisition::new` has a different signature in the actual crate, adapt the constructor call. The point of the test is the metadata mode tag, not the HTTP behavior.

- [ ] **Step 7: Run; verify fails.**

Run: `cargo test -p evalkit task_http_metadata_reports_http_mode`
Expected: FAIL (`Task::http` not defined).

- [ ] **Step 8: Implement `Task::http` and `Task::subprocess` constructors.**

In `evalkit/src/task.rs`, add:

```rust
impl Task<serde_json::Value, serde_json::Value> {
    pub fn http(plugin: evalkit_providers::HttpAcquisition) -> Self {
        // Wrap the plugin's OutputSource impl in a Task with mode = "http".
        Self::from_output_source_with_mode(plugin, "http")
    }

    pub fn subprocess(spec: evalkit_providers::SubprocessAcquisition) -> Self {
        Self::from_output_source_with_mode(spec, "subprocess")
    }
}

impl<I, O> Task<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn from_output_source_with_mode<S>(source: S, mode: &'static str) -> Self
    where
        S: OutputSource<I, O> + 'static,
    {
        let source = Arc::new(source);
        Self {
            produce: Arc::new(move |input: &I| {
                let source = Arc::clone(&source);
                let input_owned: *const I = input as *const I;
                Box::pin(async move {
                    // SAFETY: source.produce borrows input for the duration of
                    // the await; the caller guarantees input outlives this call.
                    unsafe { (*input_owned).pipe(|i| source.produce(i)) }.await
                })
            }),
            mode,
        }
    }
}
```

The `unsafe` lifetime workaround above is intentional: `OutputSource::produce` borrows `&I` and returns a future, but the boxed-future signature has to be `'static`. The simpler alternative is to require `I: Clone` and clone before await — preferred if the providers' public types allow it. **Check the `HttpAcquisition` and `SubprocessAcquisition` input types in `evalkit-providers/src/lib.rs` first.** If `I = serde_json::Value` (cloneable) is the common case, switch to:

```rust
async move {
    let owned = input.clone();
    source.produce(&owned).await
}
```

and remove the unsafe block.

(Implementer note: prefer the `Clone` path. Only fall back to unsafe lifetime extension if the providers' input types are not `Clone`. If neither works cleanly, surface the issue back and the design can be revisited — `Task` may need to consume types that already carry a `'static` produce signature, in which case the `from_fn` lifetime model is the right primitive and `from_output_source_with_mode` becomes a different shape.)

- [ ] **Step 9: Run; verify pass.**

Run: `cargo test -p evalkit task_http_metadata_reports_http_mode`
Expected: PASS.

- [ ] **Step 10: Add a positive-path `Task::subprocess` test (mirroring `http`).**

Same shape as Step 6, but for `subprocess`. Run, verify fails (it shouldn't if the constructor is in place; if it passes immediately because the implementation is symmetric, that's fine — note in commit message that the test is a regression guard).

- [ ] **Step 11: Run full workspace tests.**

Run: `cargo test --workspace`
Expected: all tests pass, including new ones.

- [ ] **Step 12: Commit.**

```bash
git add evalkit/src/task.rs evalkit/src/lib.rs
git commit -m "feat: add Task<I, O> adapter for active output sources

Task wraps closures (Task::from_fn), HTTP plugins (Task::http), and
subprocess plugins (Task::subprocess) behind one named type that
implements OutputSource. Lets users construct a typed active source
and pass it to .source(...) without reaching into the providers
crate directly."
```

---

## Task 4: Wire format — plugin protocol JSON, TOML, JSONL schema

This task updates the wire formats that travel between processes (plugin handshake JSON, run-log JSONL) and between user and CLI (TOML config). All version-bump-style changes happen here.

**Files (modify):**
- `docs/plugin-protocol.md`
- `docs/cli-config.md`
- `evalkit-cli/src/main.rs` (TOML key parsing)
- `evalkit-providers/src/lib.rs` (handshake `kind` field)
- `python/evalkit_plugin/src/evalkit_plugin/runtime.py`
- `python/evalkit_plugin/src/evalkit_plugin/__init__.py`
- `typescript/evalkit_plugin/src/runtime.ts`
- `typescript/evalkit_plugin/src/index.ts`

**Files (rename via `git mv`):**
- `docs/schema/run-log-v1.schema.json` → `docs/schema/run-log-v2.schema.json`
- `python/evalkit_plugin/examples/echo_acquisition.py` → `python/evalkit_plugin/examples/echo_source.py`
- `typescript/evalkit_plugin/examples/echo_acquisition.ts` → `typescript/evalkit_plugin/examples/echo_source.ts`

- [ ] **Step 1: Rename JSONL schema file.**

```bash
git mv docs/schema/run-log-v1.schema.json docs/schema/run-log-v2.schema.json
```

- [ ] **Step 2: Update schema JSON content.**

In `docs/schema/run-log-v2.schema.json`:
- Update top-level `$id`, `title`, or `description` field if it includes "v1" — change to "v2".
- Update the property name `acquisition_mode` to `source_mode` in the `properties` block (search for `"acquisition_mode"`; replace with `"source_mode"`).
- If the schema declares `"required": [..., "acquisition_mode", ...]`, update the entry there too.
- If there's a `"version": "1"` field at the top level, change to `"2"`.

- [ ] **Step 3: Update `docs/plugin-protocol.md`.**

Apply these edits:
- Title and intro: replace "acquisition" → "source" where it refers to plugin kind. Keep mentions of "acquisition" if they describe historical context only — but for 1.0 docs, prefer a clean cut. Replace all 19 occurrences in this file using the rules below:
  - "acquisition plugin" → "source plugin"
  - "Acquisition plugins" → "Source plugins"
  - `"kind": "acquisition"` → `"kind": "source"`
  - `[acquisition]` (TOML example) → `[source]`
  - "Acquisition Request Format" / "Acquisition Response Format" headings → "Source Request Format" / "Source Response Format"
  - `AcquisitionError::ExecutionFailed` → `OutputSourceError::ExecutionFailed`
  - `AcquisitionError::Timeout` → `OutputSourceError::Timeout`
  - "the acquisition path" → "the output source path"
- Bump the protocol version field if the doc specifies one (search for `"protocol_version"` or similar). Increment the major.

- [ ] **Step 4: Update plugin handshake `kind` value in Rust providers code.**

In `evalkit-providers/src/lib.rs`: locate the constant or string literal that emits/expects `"acquisition"` as the handshake `kind` value. Change to `"source"`. There may be multiple sites (one for HTTP handshake, one for subprocess handshake, one for validation). Use:

```bash
grep -n '"acquisition"' evalkit-providers/src/lib.rs
```

to find them. Update each.

- [ ] **Step 5: Update Python plugin SDK.**

In `python/evalkit_plugin/src/evalkit_plugin/__init__.py` and `runtime.py`:
- Find the `kind` constant or the handshake builder that writes `"kind": "acquisition"` → change to `"source"`.
- Update any docstrings or class names that include "Acquisition" — e.g., `AcquisitionPlugin` class → `SourcePlugin`. Update `__all__` exports.

- [ ] **Step 6: Rename Python example.**

```bash
git mv python/evalkit_plugin/examples/echo_acquisition.py python/evalkit_plugin/examples/echo_source.py
```

Update file contents to use the new SDK class names and the `kind: "source"` handshake.

- [ ] **Step 7: Update TypeScript plugin SDK.**

In `typescript/evalkit_plugin/src/index.ts` and `runtime.ts`:
- Find handshake `kind: "acquisition"` → `kind: "source"`.
- Update exported types/classes: `AcquisitionPlugin` → `SourcePlugin` (mirror Python).
- Update type unions and JSDoc comments.

- [ ] **Step 8: Rename TypeScript example.**

```bash
git mv typescript/evalkit_plugin/examples/echo_acquisition.ts typescript/evalkit_plugin/examples/echo_source.ts
```

Update file contents.

- [ ] **Step 9: Verify Rust builds.**

Run: `cargo check --workspace --all-targets`
Expected: clean. The schema file rename + Rust constant update from Task 2 Step 7 should now agree.

- [ ] **Step 10: Verify Rust tests.**

Run: `cargo test --workspace`
Expected: all green. The plugin protocol integration tests should now exercise the new `kind: "source"` handshake.

- [ ] **Step 11: Verify Python plugin SDK builds.**

```bash
cd python/evalkit_plugin && python -m pytest tests/ 2>/dev/null || python -c "import evalkit_plugin; print(evalkit_plugin.__all__)"
cd -
```

(If there's no pytest config, just verify imports work. If example tests exist, run them.)

- [ ] **Step 12: Verify TypeScript plugin SDK builds.**

```bash
cd typescript/evalkit_plugin && bun run tsc --noEmit 2>/dev/null || (cd ../.. && echo "typescript SDK type-check skipped")
cd -
```

- [ ] **Step 13: Commit.**

```bash
git add docs/plugin-protocol.md docs/schema/ evalkit-providers/src/lib.rs python/evalkit_plugin/ typescript/evalkit_plugin/
git commit -m "refactor!: bump plugin protocol and run-log schema for source rename

- Plugin handshake kind: \"acquisition\" -> \"source\".
- Run-log schema renamed v1 -> v2; field acquisition_mode -> source_mode.
- TOML config block [acquisition] -> [source].
- Python and TypeScript plugin SDKs updated to emit the new kind.

Pre-1.0 plugins and pre-1.0 run logs are not supported by 1.0+."
```

---

## Task 5: TOML config — CLI parsing + docs

The CLI parses `[acquisition]` from TOML config files. Update the parser, the docs, and any example TOML files.

**Files (modify):**
- `evalkit-cli/src/main.rs`
- `docs/cli-config.md`
- Any example `.toml` config files referenced by `docs/cli-config.md` or the CLI test fixtures.

- [ ] **Step 1: Find TOML key references in the CLI code.**

```bash
grep -n '"acquisition"\|\[acquisition\]\|acquisition:' evalkit-cli/src/main.rs evalkit-cli/tests/diff.rs
```

Expected: identifies each parser hook that consumes the `[acquisition]` block.

- [ ] **Step 2: Update parser to expect `[source]`.**

Replace `[acquisition]` table parsing with `[source]`. Update any `serde(rename = "acquisition")` attributes to `rename = "source"`. Update any error messages mentioning "missing [acquisition] block" → "missing [source] block".

- [ ] **Step 3: Update `docs/cli-config.md`.**

Replace all 8 occurrences of "acquisition" / "Acquisition" / `[acquisition]` per the rename. Headings: "## `[acquisition]`" → "## `[source]`"; "Exactly one acquisition mode must be configured" → "Exactly one source mode must be configured"; "HTTP acquisition" → "HTTP source"; "Subprocess acquisition plugin" → "Subprocess source plugin".

- [ ] **Step 4: Update test fixture TOML files (if any).**

```bash
find evalkit-cli/tests/ -name '*.toml' -exec grep -l 'acquisition' {} +
```

For each match, update the TOML.

- [ ] **Step 5: Run CLI tests.**

Run: `cargo test -p evalkit-cli`
Expected: all pass.

- [ ] **Step 6: Smoke-test the CLI with a real config.**

```bash
cargo run -p evalkit-cli -- --help
```

Expected: `--help` runs cleanly. If a sample command exists in `docs/cli-config.md`, run it against a sample config to confirm parsing works end-to-end.

- [ ] **Step 7: Commit.**

```bash
git add evalkit-cli/ docs/cli-config.md
git commit -m "refactor: rename TOML config block [acquisition] to [source]

Updates the CLI parser, configuration docs, and test fixtures to use
the new [source] table name. Aligns with the OutputSource rename."
```

---

## Task 6: Documentation sweep (non-archive)

Update prose, code samples, and headings in current (non-archive, non-spec, non-transcript) documentation to use the new vocabulary.

**Files (modify):** all of these are in `docs/` and not under `docs/archive/`, `docs/superpowers/`, or `transcripts/`:

- `docs/ROADMAP.md`
- `docs/benchmarks.md`
- `docs/competitive-analysis-2026-04.md`
- `docs/decisions.md`
- `docs/eval-facade.md`
- `docs/evalkit-kernel-boundary-plan.md`
- `docs/gap-analysis.md`
- `docs/integrations.md`
- `docs/root-crate-boundary-audit.md`
- `docs/root-crate-dep-surface.md`
- `docs/runtime-extraction-migration.md`
- `docs/scorers.md`
- `docs/stability.md`
- `docs/verda-competitive-analysis.md`

(Note: `docs/plugin-protocol.md` and `docs/cli-config.md` are already updated in Tasks 4-5.)

- [ ] **Step 1: For each doc file, list mentions of the rename targets.**

```bash
for f in docs/ROADMAP.md docs/benchmarks.md docs/competitive-analysis-2026-04.md docs/decisions.md docs/eval-facade.md docs/evalkit-kernel-boundary-plan.md docs/gap-analysis.md docs/integrations.md docs/root-crate-boundary-audit.md docs/root-crate-dep-surface.md docs/runtime-extraction-migration.md docs/scorers.md docs/stability.md docs/verda-competitive-analysis.md; do
  echo "=== $f ==="
  grep -n 'Acquisition\|acquisition\|acquire' "$f"
done
```

This produces a concrete checklist of edits per file.

- [ ] **Step 2: Apply per-file edits.**

For each match, apply the rename table. Pay attention to:
- API examples in code blocks: `Eval::new(...).acquire(...)` → `.source(...)`; `Run::builder().acquisition(...)` → `.source(...)`.
- Type names in prose: "the `Acquisition` trait" → "the `OutputSource` trait".
- Field names: `acquisition_mode` → `source_mode`.
- Concept words: where prose uses "acquisition" as a noun referring to the operation ("the acquisition step", "before the acquisition runs"), prefer "source" or "output source" depending on grammatical fit.
- Headings: "Acquisition" → "Output Source".

Files like `docs/decisions.md` should explicitly note the rename as a decision (one-line entry: "1.0 — `Acquisition` renamed to `OutputSource`; see `docs/superpowers/specs/2026-04-26-output-source-naming-design.md`").

- [ ] **Step 3: Spot-check the eval-facade doc for accuracy.**

`docs/eval-facade.md` is the canonical user-facing facade doc. Read it end-to-end after the renames and confirm:
- All code samples compile if pasted (verify against the actual `Eval`/`Run` APIs in the renamed code).
- The example `Eval::new(samples).source(my_task).scorer(...)` matches the actual signature.
- No leftover "acquisition" anywhere.

- [ ] **Step 4: Run a final sweep grep.**

```bash
grep -rln 'Acquisition\|acquisition\|acquire' docs/ --exclude-dir=archive --exclude-dir=superpowers
```

Expected: no matches outside historical/spec docs. Any remaining match in a current doc is a missed edit — fix.

- [ ] **Step 5: Commit.**

```bash
git add docs/
git commit -m "docs: sweep prose and samples for OutputSource rename

Updates current documentation outside docs/archive and docs/superpowers
to use the new vocabulary: OutputSource, Task, .source(...), source_mode,
OtelObserver. Adds a one-line entry to docs/decisions.md pointing at
the design spec."
```

---

## Task 7: Final verification

Comprehensive verification before declaring the rename done.

- [ ] **Step 1: Full workspace check.**

Run: `cargo check --workspace --all-targets`
Expected: clean. Zero warnings about the renamed symbols.

- [ ] **Step 2: Full workspace tests.**

Run: `cargo test --workspace`
Expected: all pass.

- [ ] **Step 3: Doctests.**

Run: `cargo test --workspace --doc`
Expected: all pass. The boundary-contract `compile_fail` doctests in `evalkit/src/lib.rs` continue to hold (those tests assert internal symbols don't leak into the root crate; they should be unaffected by the rename, but verify).

- [ ] **Step 4: Examples build.**

Run: `cargo build --workspace --examples`
Expected: all examples compile.

- [ ] **Step 5: Run smoke examples.**

```bash
cargo run -p evalkit --example basic
cargo run -p evalkit --example quickstart
```

Expected: each runs to completion and prints expected output.

- [ ] **Step 6: Run benchmarks (smoke, not perf check).**

Run: `cargo bench --workspace --no-run`
Expected: benchmarks compile. Don't run them for performance — just confirm they build.

- [ ] **Step 7: Final grep for missed renames.**

```bash
grep -rln 'Acquisition\|acquisition\|acquire' \
  --include='*.rs' --include='*.toml' --include='*.json' --include='*.py' --include='*.ts' \
  evalkit*/ python/ typescript/
```

Expected: no matches in non-archive, non-spec, non-transcript code.

If any match appears, classify:
- True missed rename: fix.
- Test name describing concept (e.g., `acquire_then_score_round_trip`): rename for consistency.
- Comment referencing past behavior: rephrase or remove.
- Genuine domain-language use unrelated to the trait (unlikely): leave with explicit comment explaining why.

- [ ] **Step 8: Final commit (only if Step 7 produced fixes).**

```bash
git add -A
git commit -m "refactor: clean up final OutputSource rename leftovers"
```

If Step 7 produced no fixes, skip this step.

- [ ] **Step 9: Squash review (optional, if user prefers a single commit).**

If the user wants the rename as one commit instead of the series of seven, use `git rebase -i` interactively to squash. **Do not pass `-i` non-interactively** (the system can't drive interactive rebase). If squashing is wanted, surface back for the user to drive it.

---

## Self-review notes

- Spec coverage check: every symbol in the spec's "Symbol rename table" appears in this plan's rename quick-reference and is touched in Task 2 or Task 3. Wire formats from the spec's Layer 5 are covered in Task 4. The `Task<I, O>` adapter from spec Layer 4 is Task 3. `OtelObserver` from the spec is Task 2 Step 12. The 1.0 stability contract section of the spec doesn't require code changes — it's a docs/CHANGELOG concern handled in Task 6.
- One known soft spot: Task 3 Step 8 has implementation uncertainty around the lifetime/`Clone` tradeoff for `Task::http` and `Task::subprocess`. The plan flags this explicitly and offers a decision branch, since which path works depends on the actual input types in `evalkit-providers` (which weren't fully read during planning). The implementer should resolve this when they get to it; if neither path is clean, surface back.
- Out of scope (not blocking 1.0): the `OutputSourceExt` composability module (deferred per spec follow-ups); additional passive source crates (`evalkit-fixtures`, log/metric backends).
