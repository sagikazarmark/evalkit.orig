# Architecture Decisions

This file records Phase 0 decisions for the `evalkit` kernel. Each entry includes one rejected alternative so later changes have context.

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
