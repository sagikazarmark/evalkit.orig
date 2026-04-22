# Roadmap Gap Analysis

This document compares the current codebase to `docs/ROADMAP.md` and identifies the next implementation slices.

## Recommendation

Phase 0 is complete. The next work should move into Phase 1.

Recommended build order:
1. Freeze and publish the run-log schema as a documented Phase 1 artifact.
2. Formalize the subprocess plugin protocol and ship reference shims.
3. Finish the remaining `evalkit-scorers-llm` work on top of the newly landed anyllm-backed primitive.

## Current State Summary

The workspace is now split for Phase 0:
- Workspace members include `evalkit`, `evalkit-cli`, `evalkit-providers`, `evalkit-exporters-langfuse`, `evalkit-scorers-text`, `evalkit-otel`, `evalkit-scorers-llm`, `evalkit-scorers-rag`, and `evalkit-scorers-embed`
- OTel support now lives in `evalkit-otel`, including the `Observe` acquisition path
- Langfuse export now lives in `evalkit-exporters-langfuse`
- Deterministic text scorers now live in `evalkit-scorers-text`
- There is no tracked `README`, but there is now a roadmap plus scorer/integration backlog docs under `docs/`

## Phase 0 - API Freeze And Kernel Features

### 0(a) Public API audit

Status: complete

Completed:
- Core kernel types are exported from `src/lib.rs`
- `AcquisitionError` is an enum with specific variants
- `ScorerError` is structured
- `ScorerContext` carries run and sample metadata
- `Score::Structured` is in the kernel
- `docs/decisions.md` records the Phase 0 decisions

### 0(b) Decisions, semver policy, multi-crate layout

Status: complete

Completed:
- `docs/ROADMAP.md` defines the workspace layout
- `docs/decisions.md` and `docs/stability.md` are in-repo
- The workspace layout is split across the Phase 0 crates

### 0(c) Workspace split

Status: complete

Current state:
- Deterministic scorers now also exist in the standalone `evalkit-scorers-text` crate
- HTTP and subprocess acquisitions now live in `evalkit-providers`
- `evalkit-exporters-langfuse` now exists as a standalone crate
- `evalkit-otel` now owns Jaeger, OTLP, and `Observe`

Gap to roadmap:
- The `evalkit-scorers-llm` crate now contains the first anyllm-backed `LlmJudge` implementation, but the broader Phase 1 scorer catalog is still incomplete.

### 0(d) Kernel features

Status: complete

Already present:
- `TrialResult` records per-trial duration
- `RunMetadata` records timing, acquisition mode, trial count, score definitions, seed, fingerprints, and explicit reproducibility metadata fields
- `stats.rs` already computes Wilson confidence intervals for binary scores
- `comparison.rs` already runs a paired t-test-style significance check for aggregate deltas
- `scorer_ext.rs` already ships `.and()`, `.or()`, `.not()`, `.map_score()`, `.timeout()`, `.weighted()`, `.then()`, and `ignore_reference`
- `SampleResult` already records `TokenUsage` and `cost_usd`
- `Score::Structured` is available

Remaining work:
- No major kernel feature gaps remain in the current Phase 0(d) checklist.
- `evalkit` is now at the Phase 0 release boundary (`0.2.0`).

## Phase 1 - Polyglot Protocol And Run-Log Schema

Status: early partials only

Already present:
- JSONL read/write helpers exist
- `evalkit-cli` already contains subprocess acquisition support
- `evalkit-otel` already contains the OTLP receiver and the `Observe` acquisition path
- `evalkit-scorers-llm` now ships a provider-neutral `LlmJudge` built on `anyllm::ChatProvider` plus structured extraction via `ExtractExt`
- The new judge implementation includes stable prompt hashing, retries, timeout support, judge model pins, and reasoning capture for numeric/binary outputs
- First-pass `llm_classifier` and `g_eval` wrappers now build on top of the shared judge primitive
- Prompt templates now live under `evalkit-scorers-llm/prompts/` and can be overridden with `LlmJudge::with_prompt`
- The kernel now aggregates scorer-side token usage and cost through `score_with_resources`, and judge token usage can flow into `SampleResult`

Gaps:
- No reference TypeScript plugin shim yet
- anyllm-backed judges currently populate token usage in `SampleResult`, but not cost because the provider layer does not expose portable cost data yet
- Reasoning capture is currently limited to numeric and binary judges because `Score::Structured` requires a numeric primary score
- `llm_classifier` is still a thin closed-set label wrapper; richer classification metadata and calibration are still missing
- `g_eval` is still a first-pass rubric prompt, not yet a fuller multi-step / auto-generated evaluation flow

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

The best next implementation slice is finishing the remaining Phase 1(c) depth:
- add portable cost reporting on top of the new scorer resource hook when provider crates can supply it
- enrich `llm_classifier` with stronger output metadata or calibration helpers
- deepen `g_eval` beyond a single rubric prompt into a more explicit step-driven evaluation flow

That sequence turns the newly landed wrappers into a more complete LLM scorer foundation instead of stopping at the first public API shape.
