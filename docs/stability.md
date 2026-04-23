# Stability Policy

This file defines the Phase 0 stability target for `evalkit` and the planned extension crates.

See also: `docs/root-crate-boundary-audit.md`

## Kernel Crate

`evalkit` is the semver anchor for the workspace.

Stable after Phase 0:
- Root exports classified as `KEEP` in `docs/root-crate-boundary-audit.md`
- `RunResult`, `SampleResult`, `TrialResult`, and `RunMetadata` field meanings
- `AcquisitionError` and `ScorerError` variant names and semantics
- Score names and serialized `Score` variant tags

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
