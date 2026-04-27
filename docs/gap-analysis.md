# Roadmap Gap Analysis

> **Note:** This document remains the current-state inventory.
>
> For the updated interpretation of that state, plus the revised kernel-boundary priority and execution order, see `docs/evalkit-kernel-boundary-plan.md`, which amends this document on that specific topic.

This document compares the current codebase to `docs/ROADMAP.md` and identifies the next implementation slices.

## Recommendation

Phases 0 through 5 now have concrete in-repo implementations. The remaining high-value work is mostly deferred or hardening-oriented rather than missing roadmap surface.

Recommended build order:
1. Exercise the GitHub Action against a live pull request environment.
2. Add deferred networked Phase 2 sources such as Kafka or NATS.
3. Deepen the server review UX beyond the current minimal annotation and dashboard surface.

## Current State Summary

The workspace is now split for Phase 0:
- Workspace members include `evalkit`, `evalkit-cli`, `evalkit-providers`, `evalkit-exporters-langfuse`, `evalkit-scorers-text`, `evalkit-otel`, `evalkit-scorers-llm`, `evalkit-scorers-rag`, `evalkit-scorers-embed`, `evalkit-scorers-redteam`, and `evalkit-server`
- OTel support now lives in `evalkit-otel`, including the `OtelObserver` source path
- Langfuse export now lives in `evalkit-exporters-langfuse`
- Deterministic text scorers now live in `evalkit-scorers-text`
- There is no tracked `README`, but there is now a roadmap plus scorer/integration backlog docs under `docs/`

## Phase 0 - API Freeze And Kernel Features

### 0(a) Public API audit

Status: complete

Completed:
- Core kernel types are exported from `src/lib.rs`
- `OutputSourceError` is an enum with specific variants
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
- HTTP and subprocess sources now live in `evalkit-providers`
- `evalkit-exporters-langfuse` now exists as a standalone crate
- `evalkit-otel` now owns Jaeger, OTLP, and `Observe`

Gap to roadmap:
- The `evalkit-scorers-llm` crate now contains the first anyllm-backed `LlmJudge` implementation, but the broader Phase 1 scorer catalog is still incomplete.

### 0(d) Kernel features

Status: complete

Already present:
- `TrialResult` records per-trial duration
- `RunMetadata` records timing, source mode, trial count, score definitions, seed, fingerprints, and explicit reproducibility metadata fields
- `stats.rs` already computes Wilson confidence intervals for binary scores
- `comparison.rs` already runs a paired t-test-style significance check for aggregate deltas
- `scorer_ext.rs` already ships `.and()`, `.or()`, `.not()`, `.map_score()`, `.timeout()`, `.weighted()`, `.then()`, and `ignore_reference`
- `SampleResult` already records `TokenUsage` and `cost_usd`
- `Score::Structured` is available

Remaining work:
- No major kernel feature gaps remain in the current Phase 0(d) checklist.
- `evalkit` is now at the Phase 0 release boundary (`0.2.0`).

## Phase 1 - Polyglot Protocol And Run-Log Schema

Status: exit criteria met, with follow-on scorer depth still open

Already present:
- JSONL read/write helpers exist
- `evalkit-cli` already contains subprocess source support
- `evalkit-otel` already contains the OTLP receiver and the `OtelObserver` source path
- `evalkit-scorers-llm` now ships a provider-neutral `LlmJudge` built on `anyllm::ChatProvider` plus structured extraction via `ExtractExt`
- The new judge implementation includes stable prompt hashing, retries, timeout support, judge model pins, and reasoning capture for numeric/binary outputs
- First-pass `llm_classifier` and `g_eval` wrappers now build on top of the shared judge primitive
- Prompt templates now live under `evalkit-scorers-llm/prompts/` and can be overridden with `LlmJudge::with_prompt`
- The kernel now aggregates scorer-side token usage and cost through `score_with_resources`, and judge token usage can flow into `SampleResult`
- `evalkit-scorers-llm::ModelPricing` now lets judge scorers estimate portable `cost_usd` from provider token usage when callers configure pricing
- `g_eval` now auto-generates explicit evaluation steps from rubric criteria and also supports caller-provided step overrides
- `g_eval_multi_pass` now performs a planning pass that drafts evaluation steps before the final scoring pass
- `llm_classifier` now accepts richer typed label definitions with optional descriptions, not just bare label strings
- classifier labels now support arbitrary per-label metadata that is preserved in prompts and calibrated structured score metadata
- `calibrated_llm_classifier` can now turn label outputs into numeric `Score::Structured` results using per-label calibration scores
- Reference TypeScript plugin shim source now lives under `typescript/evalkit_plugin/`
- `devenv.nix` now enables Bun and the TypeScript plugin shim typechecks successfully through `devenv shell`

Gaps:
- Reasoning capture is currently limited to numeric and binary judges because `Score::Structured` requires a numeric primary score

## Phase 2 - Streaming / Online Scoring

Status: roadmap-complete for the non-networked slice

Already present:
- `src/executor.rs` now introduces a first `Executor` trait plus a pull-based `PullExecutor`
- `SampleSource` and `DatasetSource` provide the first explicit source abstraction for online-style execution
- `JsonlFileTailSource` now provides a second source adapter by tailing appended JSONL `Sample` rows from disk
- `PullExecutor` now supports an optional judge-model tier that can re-score flagged samples with a secondary scorer set
- `PullExecutor` now supports checkpoint-based partial scoring for string outputs via `partial_string_scoring(...)`
- `OutputSource::produce_with_snapshots(...)` plus `SourceOutput` / `OutputSnapshot` now let output sources surface real intermediate outputs to the executor
- `PullExecutor::streaming_string_scoring(...)` now scores configured intermediate string stages from source-provided snapshots
- `ExecutionSink` and `NoopSink` provide the first sink abstraction, with per-sample notifications plus a final run completion hook
- `evalkit-otel` now provides `OtelResultSink`, so executor-based runs can emit OTel spans through the sink interface without an extra post-processing step
- `evalkit-otel` now provides `OtlpReceiverSource`, which adapts the in-repo OTLP receiver into a pull-based executor source over grouped sample spans
- `AlwaysSampler`, `PercentSampler`, and `TargetedSampler` now exist in the kernel
- `PullExecutor` now has explicit queueing and stop controls via `queue_capacity`, `max_samples`, `shutdown_when`, `ShutdownMode`, and configurable `worker_count`
- The executor path reuses existing output source timeout handling, scorer execution, score validation, judge model pin collection, and run metadata fingerprinting from the batch runner
- `examples/prod_eval_daemon.rs` now shows a small daemon-style binary composed from library primitives and emits OTel spans through `OtelResultSink`

Gaps:
- Streaming partial scoring now supports source-provided intermediate snapshots, but there is still no token-by-token or chunk-stream protocol shared across providers
- Deferred networked source adapters such as Kafka or NATS are still absent by user request
- The executor now has a bounded worker pool plus deterministic sharding, but there is still no remote coordinator or cross-process backpressure protocol

## Phase 3 - CI / Developer Workflow

Status: substantial workflow foundation landed

Already present:
- `evalkit-cli` exists as a binary crate
- `evalkit diff <baseline> <candidate>` now exists and can emit markdown plus optional JSON output from kernel comparisons
- `evalkit watch` now reruns evals on file changes and supports a bounded `--max-runs` mode for automation
- `docs/cli-config.md` now formalizes the TOML config shape used by `evalkit run` and `evalkit watch`
- `evalkit run` and `evalkit watch` now support dataset selection by `split`, `tags`, and exact-match metadata filters via the TOML config
- GitHub Action source now exists under `.github/actions/evalkit-pr-comment/` and wraps `evalkit run` plus `evalkit diff`

Gaps:
- The GitHub Action is source-only right now; it was not exercised against a live pull request from this environment

## Phase 4 - App Surface

Status: initial app surface landed

Already present:
- `evalkit-server` now exists as a standalone workspace crate
- SQLite-backed run storage now exists for `RunResult` payloads plus source sample snapshots
- HTTP API routes now cover list / get / diff / annotate / promote / alert-rule flows
- Minimal server-rendered web UI now covers run browsing, run drill-down, diff viewing, and dashboard pages
- Annotation flow now writes promoted dataset JSONL entries back out from stored source samples
- Threshold alert rules and a basic dashboard now exist on top of stored runs
- The server UI now includes a richer filterable review queue with review summaries, quick triage actions, and source-sample context on each reviewed item
- Stored runs can now also carry sample output playback, including per-trial outputs and intermediate snapshots for review
- The diff UI now includes side-by-side sample adjudication with baseline/candidate playback and per-scorer change summaries

Gaps:
- The current UI is still intentionally server-rendered; it does not yet expose threaded collaboration or background review jobs
- Dashboard alerts currently evaluate stored run results rather than directly owning OTLP ingestion inside the server process
- There is still no authentication, multi-user workflow, or background job model

## Phase 5 - Scale And Governance

Status: initial exit criteria met

Completed:
- `ShardSpec` plus `ShardedSource<Src>` now provide deterministic source sharding for distributed-style fanout
- `PullExecutor::scrubber(...)` plus `Scrubber`, `NoopScrubber`, and `RegexPiiScrubber` now add PII scrubbing hooks at the eval pipeline boundary
- `evalkit-server::RunStore::detect_drift(...)` now compares recent stored runs and surfaces drift through both the dashboard and `/api/runs/:run_id/drift`
- `evalkit-scorers-redteam` now provides a first adversarial scorer pack with heuristic `toxicity`, `bias`, `pii_leakage`, `misuse`, `jailbreak_detected`, and aggregate `policy_adherence`

Gaps:
- Sharding is deterministic and library-level; there is still no remote scheduler or worker coordination service
- Drift detection currently uses trailing-window heuristics over stored run stats rather than a general event-stream drift engine
- The red-team pack is intentionally heuristic; richer judge-backed safety scorers remain future work

## Highest-Leverage Next Slice

With the current roadmap phases implemented in-repo, the next high-leverage slices are mostly the explicitly deferred and hardening-oriented items:
- live GitHub Action validation in a real PR environment
- deferred networked Phase 2 sources such as Kafka or NATS
- richer review / triage workflow on top of the existing server dashboard and annotation flow
