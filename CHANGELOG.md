# Changelog

## 2.0.0 — Kernel Output API Redesign

Bundled redesign of the kernel output surface. Schema-breaking; bumps `RUN_RESULT_SCHEMA_VERSION` from `"2"` to `"3"`. See `docs/superpowers/specs/2026-04-27-kernel-output-api-2.0-design.md` for the design rationale.

### Breaking changes

**`OutputSource::produce` returns an envelope.**

```rust
// before
async fn produce(&self, input: &I) -> Result<O, OutputSourceError>;

// after
async fn produce(&self, input: &I) -> Result<ProductionOutput<O>, OutputSourceError>;
```

`ProductionOutput<O>` carries `output`, optional `usage` (`TokenUsage`), `cost_usd`, `latency`, and a freeform `metadata` map. The closure blanket impl wraps bare `Result<O, _>` via `ProductionOutput::new` — closure-based callers don't need to update.

**`Score::Structured` removed.** Reasoning and structured metadata move from inside the `Score` variant onto `ScoreOutcome`:

```rust
// before
ScoreOutcome::new(Score::Structured { score: 0.9, reasoning, metadata })

// after
ScoreOutcome::new(Score::Numeric(0.9)).with_reasoning(reasoning)
```

Label and binary judges newly support reasoning capture.

**`TrialResult.scores` is `HashMap<String, ScoredEntry>`** (was `HashMap<String, Result<Score, ScorerError>>`). `ScoredEntry { result, reasoning, metadata }`.

**`ScorerResources` renamed to `ResourceUsage`** with a new `latency: Option<Duration>` field. Used on both source and scorer sides.

**`ScorerContext` gains `seed`, `cancel`, `budget`, `previous_scores`.** `seed` is `Option<u64>` threaded from `Run::seed`. `cancel` is a `tokio_util::sync::CancellationToken` — scorers can check `ctx.cancel.is_cancelled()` cooperatively. `budget` is an optional `&Budget` for advisory cost ceilings. `previous_scores` is a map of upstream successful scores within a `ScorerSet` (sequential execution, declaration order).

**`SampleResult` adds `source_resources` and `scorer_resources`** — separate accumulation of source-side and scorer-side resources. The unified `token_usage` and `cost_usd` fields remain as their union for back-compat reads.

**`OutputSource::metadata()` replaced by `metadata_mode()`.** Returns `&'static str` directly instead of a `SourceMetadata` struct.

**`OutputSourceError`:**
- `TraceNotFound` variant removed (moved to `evalkit-otel` as `OtelTraceNotFound`, wrapped in `ExecutionFailed`).
- `Panicked` carries a `String` payload with the panic message.
- New `is_retryable()` method; default true for `BackendUnavailable` and `Timeout`.

**Snapshots moved out of the kernel.** `OutputSnapshot`, `SourceOutput`, and `produce_with_snapshots` live in `evalkit-runtime` as a `SnapshotSource` extension trait. `PullExecutor::streaming_string_scoring` now requires a `streaming_source` to be set; misconfiguration produces `ExecutorError::Configuration`.

**Schema bump from `"2"` to `"3"`** — old JSONL files are rejected by the v3 reader. Use `evalkit migrate-runlog --in <file> --out <file>` to upgrade. `evalkit_server::migrate_storage_v2_to_v3(connection)` does the same for stored runs.

### Internal cleanups

- Four `RunExecutor` variants collapsed into one `MappedRunExecutor`. Type-state machine on the `RunBuilder` is preserved.
- `RunStats.mixed_variant_scorers: Vec<String>` surfaces scorers that emit mixed `Score` variants instead of silently dropping them.

### Decisions deferred

- The v3 JSON schema document (`docs/schema/run-log-v3.schema.json`) is partially updated — the schema version constant is bumped but the full `TrialResult` / `SampleResult` shape updates are tracked as a follow-up.
- Surfacing `usage` / `cost_usd` / `latency` from HTTP and subprocess plugin responses requires a plugin-protocol bump and is deferred.
- The `current_sample_id` task-local stays in the kernel rather than moving to `evalkit-otel` — moving it would invert the `evalkit-runtime` / `evalkit-otel` layering. See `docs/decisions.md` for details.
