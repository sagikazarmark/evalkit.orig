# Roadmap Gap Analysis

This document compares the current codebase to `docs/ROADMAP.md` and identifies the next implementation slices.

## Recommendation

The next work should stay inside Phase 0 and focus on the kernel API before any crate split.

Recommended build order:
1. Land the Phase 0 decision and stability docs.
2. Freeze the kernel surface by adding `Score::Structured`, expanding `ScorerContext`, and replacing the `ScorerError` newtype with structured variants.
3. Add the remaining scorer composition operators: `.or()`, `.not()`, `.map_score()`, and `.timeout()`.
4. Expand `RunMetadata` and per-sample results with reproducibility and usage tracking.
5. Split the monolith into focused crates only after the shared types stop moving.

## Current State Summary

The current repository is still mostly pre-split:
- Workspace members: `evalkit`, `evalkit-cli`
- Feature-gated modules still live inside the kernel for OTel, Langfuse, and the LLM judge
- There is no tracked `README`, but there is now a roadmap plus scorer/integration backlog docs under `docs/`

## Phase 0 - API Freeze And Kernel Features

### 0(a) Public API audit

Status: partial

Already present:
- Core kernel types are exported from `src/lib.rs`
- `AcquisitionError` is already an enum with specific variants
- `Sample` and `Dataset` already carry metadata maps

Gaps:
- `ScorerError` is still a boxed newtype
- `ScorerContext` only carries input, output, and reference
- `Score` has no structured variant
- There is no tracked decision log until this change

### 0(b) Decisions, semver policy, multi-crate layout

Status: newly started with this change

Already present:
- `docs/ROADMAP.md` defines the intended workspace layout

Gaps:
- `docs/decisions.md` and `docs/stability.md` were missing
- The current workspace was still monolithic, with only the root crate and `evalkit-cli`

### 0(c) Workspace split

Status: partial

Current state:
- Deterministic scorers now also exist in the standalone `evalkit-scorers-text` crate
- HTTP and subprocess acquisitions now live in `evalkit-providers`
- `evalkit-exporters-langfuse` now exists as a standalone crate
- `evalkit-otel` now exists for Jaeger and OTLP backends, while the umbrella crate still carries the `Observe` compatibility path

Gap to roadmap:
- The umbrella crate still keeps the deterministic scorer implementations in-kernel for compatibility during the split
- The umbrella crate still carries transitional feature-gated Langfuse and OTel modules during the split

### 0(d) Kernel features

Status: partial

Already present:
- `TrialResult` records per-trial duration
- `RunMetadata` records timing, acquisition mode, trial count, score definitions, seed, fingerprints, and explicit reproducibility metadata fields
- `stats.rs` already computes Wilson confidence intervals for binary scores
- `comparison.rs` already runs a paired t-test-style significance check for aggregate deltas
- `scorer_ext.rs` already ships `.and()`, `.or()`, `.not()`, `.map_score()`, `.timeout()`, `.weighted()`, `.then()`, and `ignore_reference`
- `SampleResult` already records `TokenUsage` and `cost_usd`
- `Score::Structured` is available

Missing or incomplete:
- No automatic population of code identity or judge model pins beyond explicit builder inputs
- No explicit Wilcoxon fallback for paired comparisons

## Phase 1 - Polyglot Protocol And Run-Log Schema

Status: early partials only

Already present:
- JSONL read/write helpers exist
- `evalkit-cli` already contains subprocess acquisition support
- The kernel already has an OTLP receiver behind the `otel` feature

Gaps:
- No published schema module or versioned JSON Schema document
- No `docs/plugin-protocol.md`
- No formal scorer plugin protocol or conformance suite
- Current `llm_judge` uses ad hoc HTTP instead of `anyllm`
- No `TrajectorySample`, `ConversationSample`, or shared `ToolCall` type
- No OTel eval-result emitter

## Phase 2 - Streaming / Online Scoring

Status: not started

Gaps:
- No `Executor` trait
- No online sources such as Kafka, NATS, or file tailing
- No partial-stream scoring path
- No streaming sink abstraction

## Phase 3 - CI / Developer Workflow

Status: partial CLI foundation only

Already present:
- `evalkit-cli` exists as a binary crate

Gaps:
- No `evalkit diff`
- No `evalkit watch`
- No GitHub Action
- No formal CLI config spec

## Phase 4 - App Surface

Status: not started

Gaps:
- No `evalkit-server`
- No SQLite-backed run store
- No web UI
- No annotation flow

## Phase 5 - Scale And Governance

Status: not started

Gaps:
- No distributed execution
- No PII scrubbing hooks
- No drift detection
- No red-team scorer pack

## Highest-Leverage Next Slice

The best next implementation slice is still a kernel-only one:
- add structured scores
- add richer scorer context
- add structured scorer errors
- add the missing composition operators

That sequence gives the future split and the future plugin protocol a stable surface to build on, while keeping the current monolithic crate usable.
