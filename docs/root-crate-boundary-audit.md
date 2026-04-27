# Root Crate Boundary Audit

Date: 2026-04-23

This document closes the audit step from `docs/issues/01-audit-root-crate-boundary.md`.

Its job is simple: every public root export in `src/lib.rs` is classified as either:

- keep in `evalkit` as kernel surface
- move out of the root crate during the split
- explicit decision recorded here

This document is the source of truth for what the root crate is supposed to be.

## Design target

`evalkit` stays the semver anchor and the easiest batch-oriented kernel entrypoint.

It does not stay the home for runtime orchestration, file tailing, sampling policy, or AI-specific sample-shape conveniences.

## Current problem

Today `src/lib.rs` exports three different kinds of things from one place:

- true kernel concepts
- runtime and operational helpers
- AI-shaped convenience types

That is how a root crate quietly becomes a junk drawer.

## Classification summary

```text
KEEP IN evalkit (kernel)
  OutputSource / Dataset / Mapper / Run / RunResult / Sample /
  Score / ScoreDefinition / Scorer / ScorerSet / Comparison /
  schema version / stats / scorer composition

MOVE OUT OF evalkit (runtime or optional surface)
  Executor / PullExecutor / Sources / Sinks / Samplers /
  Scrubbers / Sharding / file-tail helpers / current_sample_id /
  JSONL IO helpers / conversation + trajectory sample shapes
```

## Export-by-export audit

### Keep in `evalkit`

These are kernel surface and should remain re-exported from the root crate.

| Export(s) | Decision | Why |
|---|---|---|
| `SourceOutput`, `OutputSource`, `OutputSourceError`, `SourceMetadata`, `OutputSnapshot` | KEEP | OutputSource is part of the core Dataset -> OutputSource -> Scorer pipeline. |
| `Change`, `CompareConfig`, `Comparison`, `SampleComparison`, `ScorerComparison`, `compare` | KEEP | Run-to-run comparison is a core kernel capability, not an optional runtime concern. |
| `Dataset` | KEEP | Core data model. |
| `MapError`, `Mapper` | KEEP | Shared transforms are part of the core scorer pipeline. |
| `Run`, `RunBuildError`, `RunError` | KEEP, with cleanup | `Run` stays the stable batch entrypoint, but its internals must shed runtime-heavy implementation debt. |
| `RunMetadata`, `RunResult`, `SampleResult`, `TokenUsage`, `TrialResult` | KEEP | Core result and reporting model. |
| `Sample`, `SampleBuildError`, `SampleBuilder` | KEEP | Core data model. |
| `RUN_RESULT_SCHEMA_VERSION` and `pub mod schema` | KEEP | Schema versioning is kernel-level contract surface. |
| `Score` | KEEP | Core scoring model. |
| `Direction`, `ScoreDefinition` | KEEP | Core scoring model. |
| `ScoreOutcome`, `Scorer`, `ScorerMetadata`, `ScorerResources` | KEEP | Core scoring traits and scorer-side resource accounting. |
| `ScorerContext` | KEEP | Core scorer execution context. |
| `ScorerError` | KEEP | Core scorer error taxonomy. |
| `AndScorer`, `IgnoreReferenceScorer`, `MapScoreScorer`, `NotScorer`, `OrScorer`, `ScorerExt`, `ThenScorer`, `TimeoutScorer`, `WeightedScorer`, `ignore_reference` | KEEP | Scorer composition is part of the kernel API, not a plugin layer. |
| `ScorerSet` | KEEP | Core scorer grouping abstraction. |
| `RunStats`, `ScorerStats` | KEEP | Statistical aggregation is one of the kernel's main differentiators. |

### Move out of `evalkit`

These should not remain root-crate exports after the split.

| Export(s) | Destination | Why |
|---|---|---|
| `AlwaysSampler`, `PercentSampler`, `TargetedSampler`, `Sampler`, `SamplerBuildError` | `evalkit-runtime` | Runtime sampling policy, not kernel semantics. |
| `DatasetSource`, `SampleSource`, `ExecutionSink`, `NoopSink` | `evalkit-runtime` | Runtime ingestion and emission boundaries. |
| `Executor`, `PullExecutor`, `ExecutorBoxError`, `ExecutorError` | `evalkit-runtime` | Online execution runtime, not batch kernel. |
| `JsonlFileTailSource` | `evalkit-runtime` | File-tail ingestion is operational surface. |
| `NoopScrubber`, `RegexPiiScrubber`, `Scrubber` | `evalkit-runtime` | Scrubbing is runtime policy, not kernel semantics. |
| `ShardBuildError`, `ShardSpec`, `ShardedSource` | `evalkit-runtime` | Sharding belongs to execution/runtime orchestration. |
| `ShutdownMode`, `StringPrefixCheckpoint`, `StringStreamStage` | `evalkit-runtime` | Partial scoring and shutdown controls are runtime-specific. |
| `current_sample_id` | internal or `evalkit-runtime` | Task-local execution helper, not user-facing kernel concept. |
| `read_jsonl`, `write_jsonl` | future `evalkit-exporters-jsonl` or similar IO crate | File-format IO is operational convenience, not kernel contract. |
| `ConversationSample`, `ConversationTurn`, `ToolCall`, `ToolResult`, `TrajectorySample`, `TrajectoryStep` | future optional sample-shapes crate | AI-shaped convenience types should not define the generic kernel. |

## Explicit decision log

These were the slippery items from the earlier plan. They are now decided.

### `Run`

Decision: keep `Run` in the root crate.

Why:
- The kernel needs one stable, obvious batch entrypoint.
- Moving `Run` out would leave `evalkit` with types but no natural orchestration surface.
- The problem is not that `Run` exists. The problem is that the current implementation still pulls in runtime-heavy details.

Required follow-up:
- keep the public `Run` type in `evalkit`
- remove or isolate implementation details that force a runtime-heavy kernel story
- specifically re-evaluate timeout handling, code-fingerprint plumbing, and any other operational behavior that does not belong in the long-term root crate

### `read_jsonl` / `write_jsonl`

Decision: move out of the root crate.

Why:
- The schema version constant is kernel contract.
- Reading and writing files is not.
- File-format convenience APIs are useful, but they belong in an IO/exporter layer.

Required follow-up:
- keep `RunResult` and schema types in the kernel
- move JSONL reader/writer helpers to a dedicated IO/exporter crate during the split

### `ConversationSample` / `TrajectorySample`

Decision: move out of the root crate.

Why:
- These are useful AI-facing shapes.
- They are not generic kernel primitives.
- Leaving them in the root crate weakens the claim that `evalkit` is domain-agnostic at the core.

Required follow-up:
- move the full family together: `ConversationSample`, `ConversationTurn`, `ToolCall`, `ToolResult`, `TrajectorySample`, `TrajectoryStep`
- keep plain `Sample<I, R>` as the generic kernel primitive

## ASCII boundary diagram

```text
ROOT CRATE TODAY                                ROOT CRATE AFTER SPLIT

evalkit                                         evalkit
├── Dataset / Sample / Score                    ├── Dataset / Sample / Score
├── Scorer / ScorerSet / Mapper                 ├── Scorer / ScorerSet / Mapper
├── Run / RunResult / Comparison / Stats        ├── Run / RunResult / Comparison / Stats
├── OutputSource                                ├── OutputSource
├── JSONL IO helpers                 X          ├── schema version only
├── conversation / trajectory shapes X          └── no runtime orchestration exports
└── runtime executor surface         X

                                                evalkit-runtime
                                                ├── Executor / PullExecutor
                                                ├── Sources / Sinks
                                                ├── Samplers / Scrubbers
                                                ├── Sharding / shutdown / stream helpers
                                                └── operational execution helpers

                                                optional crates
                                                ├── JSONL IO / exporters
                                                └── sample-shape conveniences
```

## Stability impact

This audit changes how to read `docs/stability.md`.

Not every current root export should be treated as part of the intended long-term kernel.

From this point on:
- the `KEEP` exports above are the intended stable kernel surface
- the `MOVE` exports above are transitional root exports that should leave the crate during the split
- the root prelude should shrink with the root crate, not preserve transitional exports forever

## Short version

Keep `Run`.

Move runtime, JSONL IO, and AI-shaped sample conveniences out of the root crate.

That is the boundary reset.
