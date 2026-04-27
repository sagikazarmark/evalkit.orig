# Stability Policy

This file defines the Phase 0 stability target for `evalkit` and the planned extension crates.

See also: `docs/root-crate-boundary-audit.md`

## Kernel Crate

`evalkit` is the semver anchor for the workspace.

Stable after Phase 0:
- Root exports classified as `KEEP` in `docs/root-crate-boundary-audit.md`
- `RunResult`, `SampleResult`, `TrialResult`, and `RunMetadata` field meanings
- `OutputSourceError` and `ScorerError` variant names and semantics (both are `#[non_exhaustive]`; new variants are additive)
- Score names and serialized `Score` variant tags

## Frozen API Decisions

These decisions are recorded so they cannot drift before 1.0:

- **`Scorer<I, O, R>` default reference type.** `R = ()`. Scorers that do not need a reference inherit the default and ignore `ctx.reference`. Scorers that require a reference pattern-match on `Option<&'a R>` and return `ScorerError::InvalidInput` when `None`. We do not promote `Option<R>` into the trait, and we do not use a separate `ReferenceRequired` marker trait — both add type-system noise without solving a problem the current shape doesn't already solve.
- **`Score::Metric { unit: Option<String> }`.** Free-form unit string, not a structured `Unit` enum. Unit semantics belong to the scorer that emitted the metric; a structured enum would either be incomplete or force every consumer to handle units they don't care about. Convention: SI-style suffixes for units that have them (`"ms"`, `"tokens"`, `"USD"`, `"requests/s"`).
- **`Score::Structured { score, reasoning, metadata }`.** Fixed shape: a numeric score, a free-text reasoning string, and a JSON metadata bag. LLM-judge scorers populate `reasoning` from the model's explanation; arbitrary scorer state goes in `metadata`. We do not use a heterogeneous variant per scorer family.
- **`ScorerContext` carries `run_id`, `sample_id`, `trial_index`, `metadata`, plus `input`, `output`, and `Option<&reference>`.** The `with_scope(...)` constructor is the path used by the runtime; `new(...)` exists for tests with empty scope. `metadata` is a `HashMap<String, Value>` for forward-compatible scorer hints — keys here are not part of the stable surface.
- **`Run` vs `Executor`.** `Run` stays as the batch-evaluation entry point for 1.0. A separate `Executor` trait may appear later for streaming; that decision is deferred to Phase 2. Adding `Executor` does not break the `Run` API.

Unstable before Phase 0 exit:
- Root exports classified as `MOVE` in `docs/root-crate-boundary-audit.md`
- Any API that is present only behind cargo feature flags that will be extracted into dedicated crates
- Internal helper types not re-exported from the crate root

Transitional root exports:
- Runtime-oriented executor, source, sink, sampler, scrubber, sharding, and stream-helper APIs are transitional and should leave `evalkit` during the split.
- `read_jsonl` / `write_jsonl` are transitional convenience APIs and should move to an IO/exporter layer.
- Conversation and trajectory sample-shape types are transitional convenience APIs and should move out of the root crate.

Semver rules:
- Removing or renaming a public item is a breaking change.
- Adding non-`#[non_exhaustive]` enum variants is a breaking change.
- Adding optional struct fields that change serialized output is a breaking change for persisted formats unless versioned separately.

## Extension Crates

Planned extension crates follow their own crate-level semver once extracted:
- `evalkit-scorers-*`
- `evalkit-providers`
- `evalkit-otel`
- `evalkit-exporters-*`
- `evalkit-cli`
- `evalkit-server`

Rules:
- Extension crates may release independently from the kernel.
- Extensions must not widen kernel error semantics by inventing incompatible meanings for shared kernel types.
- Crates that are experimental on first release should document that status in their own README and API docs rather than weakening the kernel guarantees.

## Run-Log Schema

The run-log schema version is independent from crate versions.

Rules:
- Schema versions use their own major versioning.
- Readers must fail loudly on newer major schema versions.
- Additive schema changes that preserve existing readers may bump a minor schema version.
- A crate release may ship multiple schema readers, but writers should default to the latest stable schema version.

## Workspace After The Split

Phase 0 ends with the split complete:
- `evalkit` keeps the kernel only.
- Specialized capabilities live in their own crates rather than behind root-crate feature flags.
- The umbrella crate does not hide specialized crates from advanced callers.
- The root crate boundary matches `docs/root-crate-boundary-audit.md`.
