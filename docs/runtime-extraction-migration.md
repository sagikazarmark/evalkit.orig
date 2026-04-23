# Runtime Extraction: Migration Notes

Closes issue `02-create-evalkit-runtime-and-move-runtime-apis.md`.

## Release mechanics

This extraction lands as a **breaking minor bump**: `evalkit 0.2.0` → `evalkit 0.3.0`.

The root crate `evalkit` has no released stable major yet (`0.x`), so under Cargo
semver a minor version bump already signals a breaking change. Callers must
update their dependency declarations.

No temporary re-export shims are kept in the root crate. The runtime APIs are
available exclusively through the new `evalkit-runtime` crate.

## What moved

Every public item listed as `MOVE` in `docs/root-crate-boundary-audit.md`
under the "runtime sampling policy / ingestion / execution / sharding /
partial-stream" sections now lives in `evalkit-runtime`:

- `Executor`, `PullExecutor`, `ExecutorBoxError`, `ExecutorError`
- `SampleSource`, `DatasetSource`, `JsonlFileTailSource`
- `ExecutionSink`, `NoopSink`
- `Sampler`, `AlwaysSampler`, `PercentSampler`, `TargetedSampler`, `SamplerBuildError`
- `Scrubber`, `NoopScrubber`, `RegexPiiScrubber`
- `ShardSpec`, `ShardedSource`, `ShardBuildError`
- `ShutdownMode`, `StringPrefixCheckpoint`, `StringStreamStage`
- `current_sample_id`

`read_jsonl` / `write_jsonl` and the `ConversationSample` / `TrajectorySample`
families are still exported from the root crate in this release. They remain
classified as `MOVE` in the audit and will land in separate follow-up crates.

## Migration for callers

For every moved item, change the import from

```rust
use evalkit::{Executor, PullExecutor, DatasetSource, AlwaysSampler};
```

to

```rust
use evalkit_runtime::{Executor, PullExecutor, DatasetSource, AlwaysSampler};
```

Add `evalkit-runtime` to `Cargo.toml`:

```toml
[dependencies]
evalkit = "0.3"
evalkit-runtime = "0.1"
```

No API signatures changed. Only the path to reach them did.

## Kernel-side changes

- `pub mod acquisition` is now exposed on the root crate so that
  `evalkit-runtime` can reach `current_sample_id` and `with_current_sample_id`.
  Their semantics are unchanged.
- `ScorerSet::score`, `ScorerSet::definitions`, and `ScorerSet::judge_model_pins`
  are now public (previously `pub(crate)`). They are the entry points runtime
  orchestration layers use to drive a scorer set, and they cannot stay private
  once the runtime lives in a sibling crate.
- The root `prelude` no longer re-exports any of the moved types.

## Verification

- `cargo build --workspace` — all workspace crates build.
- `cargo test -p evalkit` — kernel tests pass.
- `cargo test -p evalkit-runtime` — runtime tests (formerly kernel executor
  tests) pass unchanged.
- `cargo build --example prod_eval_daemon` — example now imports from
  `evalkit-runtime` and compiles.
