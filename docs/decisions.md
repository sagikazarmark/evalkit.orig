# Architecture Decisions

This file records Phase 0 decisions for the `evalkit` kernel. Each entry includes one rejected alternative so later changes have context.

## 2026-04-22 - Add `score_with_resources` instead of changing `Scorer::score` return types

Decision:
Keep `Scorer::score(&ScorerContext) -> Result<Score, ScorerError>` as the stable primary trait method, and add a default `score_with_resources(&ScorerContext) -> Result<ScoreOutcome, ScorerError>` hook for scorers that need to report token usage or cost.

Why:
- This lets `Run` aggregate scorer-side `token_usage` and `cost_usd` into `SampleResult` without breaking every existing scorer implementation.
- Wrapper scorers in `scorer_ext.rs` can override the richer hook to preserve usage through composition.
- LLM judge scorers can now surface provider token usage in kernel results while keeping the simple `Score` API intact for callers that do not care about resource accounting.

Rejected alternative:
Change `Scorer::score` itself to return a richer result object. That would be cleaner in isolation, but it would force a broad API break across the kernel, built-in scorers, examples, and downstream implementations for a problem that a default companion method solves with much smaller churn.

## 2026-04-22 - Build `evalkit-scorers-llm` on `anyllm::ChatProvider` plus `ExtractExt`

Decision:
Use `anyllm`'s provider-neutral `ChatProvider` trait as the integration seam for LLM-as-a-Judge scoring, and use the `extract` feature's `ExtractExt::extract(&ChatRequest)` path for structured score extraction.

Confirmed API shape from `anyllm` 0.1.1:
- `ChatProvider` is the core trait.
- The one-shot call path is `provider.chat(&request).await?`.
- Structured extraction lives behind the `extract` feature and is exposed as `provider.extract::<T>(&request).await?`.
- `ChatRequest::new(model)` plus `.user(...)`, `.temperature(...)`, `.max_tokens(...)`, `.seed(...)`, and `.reasoning(...)` covers the portable request shape we need.

Why:
- This keeps `evalkit-scorers-llm` provider-neutral instead of re-introducing an OpenAI-compatible HTTP client surface.
- `ExtractExt` uses anyllm's native structured-output mode when supported and falls back to a forced-tool strategy when needed, which satisfies the roadmap requirement to avoid freeform score parsing.
- `DynChatProvider` gives the scorer crate a concrete, non-generic storage type while still accepting arbitrary providers at construction time.

Rejected alternative:
Rebuild the old judge path around direct HTTP requests and ad hoc JSON parsing. That would duplicate provider logic anyllm already centralizes and would violate the Phase 1 requirement that scores come from structured output rather than freeform text parsing.

## 2026-04-22 - Keep `Scorer<I, O, R = ()>` with optional references in context

Decision:
Keep the trait shape `Scorer<I, O, R = ()>` and continue representing an absent reference as `ScorerContext::reference: Option<&R>`.

Why:
- The existing generic parameter keeps reference-bearing and reference-free scorers type-safe.
- The optional reference already models the runtime case where a dataset omits a reference.
- This is the smallest change that preserves current ergonomics and avoids pushing `Option<R>` into every scorer implementation.

Rejected alternative:
Move the optionality into the trait contract itself with `Scorer<I, O, Option<R>>`. This would leak transport concerns into every scorer signature and make simple deterministic scorers noisier without buying additional correctness.

## 2026-04-22 - Expand `ScorerContext` with run and sample metadata

Decision:
`ScorerContext` should carry `run_id`, `sample_id`, `trial_index`, and `metadata: &HashMap<String, Value>` in addition to `input`, `output`, and `reference`.

Why:
- Judge-backed scorers and exporters need stable run/sample identifiers for traces and annotations.
- Sample metadata is already present on `Sample`, so passing it through the context avoids ad hoc plumbing later.
- The additional fields are read-only and do not complicate existing scorer implementations.

Rejected alternative:
Keep these fields only on `TrialResult`. That would force scorers to rediscover run context through side channels and makes trace correlation harder.

## 2026-04-22 - Add `Score::Structured { score, reasoning, metadata }`

Decision:
Add a structured score variant with explicit fields:

```rust
Score::Structured {
    score: f64,
    reasoning: String,
    metadata: serde_json::Value,
}
```

Why:
- LLM-judge results need a canonical place for a primary numeric score and textual justification.
- Deterministic scorers can still attach rich payloads through `metadata`.
- This keeps the common case inspectable without requiring every consumer to know scorer-specific JSON layouts.

Rejected alternative:
Use `Score::Structured(serde_json::Value)`. It is more flexible, but it gives up a stable kernel-level place for the representative score and reasoning that downstream tools will want to display and compare.

## 2026-04-22 - Freeze `AcquisitionError` variants as the kernel baseline

Decision:
Treat the current `AcquisitionError` variants as the stable Phase 0 baseline:
- `ExecutionFailed`
- `TraceNotFound`
- `BackendUnavailable`
- `Timeout`
- `Panicked`

Why:
- The set already covers the acquisition failure modes present in the kernel and OTLP-backed paths.
- Freezing this shape now lets provider crates target a single error vocabulary.

Rejected alternative:
Continue adding variants opportunistically during feature work. That would make the pre-split API unstable and push semver churn into every downstream provider.

## 2026-04-22 - Enrich `ScorerError` into structured variants

Decision:
Replace the newtype `ScorerError(Box<dyn Error>)` with a structured enum that distinguishes invalid input, timeouts, provider failures, and internal errors while preserving source chains.

Why:
- The current wrapper makes it hard for callers to react differently to invalid data, upstream provider failures, and library bugs.
- Composition operators and future judge scorers need stable categories for propagation and reporting.

Rejected alternative:
Keep the boxed newtype and rely on string matching. That keeps the implementation simple but makes callers brittle and undermines the roadmap's goal of a stable kernel API.

## 2026-04-22 - Keep `Run` batch-oriented and introduce `Executor` later

Decision:
Keep `Run` focused on batch execution and introduce a separate `Executor` trait in Phase 2 for streaming and online scoring.

Why:
- The current `Run` API is batch-shaped throughout construction, execution, and result materialization.
- Streaming introduces backpressure and partial-result semantics that would overload the existing type.

Rejected alternative:
Grow `Run` into the streaming abstraction. That would couple batch and online concerns before the streaming design is tested.

## 2026-04-22 - Keep `Score::Metric.unit` as `Option<String>` for now

Decision:
Keep `Score::Metric { unit: Option<String> }` in Phase 0.

Why:
- The current kernel does not yet have enough real metric producers to justify freezing a structured unit taxonomy.
- A string keeps exporters and integrations flexible while the crate split happens.

Rejected alternative:
Introduce a `Unit` enum immediately. That risks baking in an incomplete unit catalog before token, cost, latency, and retrieval metrics have all landed.

## 2026-04-23 - Start Phase 2 with a pull-based `Executor` plus source/sampler/sink traits

Decision:
Introduce a separate `Executor` trait for online execution and start with a minimal pull-based implementation:
- `PullExecutor`
- `SampleSource` / `DatasetSource`
- `ExecutionSink`
- `AlwaysSampler`, `PercentSampler`, and `TargetedSampler`

Why:
- This keeps the existing `Run` type batch-focused while proving the Phase 2 execution seams against real code.
- A pull-based executor is the smallest production-usable shape that can reuse the kernel's existing acquisition, scoring, timeout, metadata, and fingerprinting logic.
- Separating source, sampler, and sink gives later source adapters, targeted rescoring, and streaming emitters stable insertion points without prematurely freezing queueing or concurrency policy.

Rejected alternative:
Build the first Phase 2 API around a fully concurrent queued worker system with backpressure controls from day one. That may still be the right long-term runtime, but it would force queue semantics, shutdown behavior, and threading policy into the API before the simpler source/sampler/sink boundaries have been tested.

## 2026-04-23 - Add one optional secondary judge-model tier to `PullExecutor`

Decision:
Keep the first judge-model tiering design minimal inside `PullExecutor`:
- a primary `ScorerSet` still runs for every sampled item
- one optional secondary `ScorerSet` may run for a subset of items
- the subset is chosen by a predicate over the primary scores for the current sample

Why:
- This lands the roadmap's cheap-then-expensive rescoring workflow without introducing a separate execution planner API yet.
- Predicate-based gating keeps the first version flexible enough for score-threshold, failure-only, and metadata-aware escalation policies.
- Leaving non-triggered tier scores absent from a sample's trial results matches the kernel's existing stats and comparison behavior, which already tolerate missing scorer entries.

Rejected alternative:
Require every tiered scorer to emit an explicit synthetic "skipped" score or error when the expensive tier does not run. That would complicate the score model and pollute run outputs before there is evidence that consumers need a first-class skipped state.
