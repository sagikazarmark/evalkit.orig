# Issue: Create `evalkit-runtime` and move runtime-oriented APIs out of the root crate

Suggested labels: `architecture`, `refactor`, `kernel-boundary`, `high-priority`

## Summary

Introduce `evalkit-runtime` and move the clearly runtime-oriented APIs out of `evalkit`.

Start with the surfaces already identified as Bucket B in the boundary audit.

## Why this matters

The product story is "small Rust-native eval kernel." That story does not survive if the semver anchor still exports online execution, samplers, sharding, sinks, and stream helpers from the root crate.

## Scope

- create an `evalkit-runtime` crate in the workspace
- move runtime-oriented APIs out of `evalkit`, including:
  - `Executor`
  - `PullExecutor`
  - samplers
  - sources and sinks
  - sharding
  - scrubbers
  - partial-stream and checkpoint helpers
- update examples so runtime-heavy examples import the new crate directly
- stop re-exporting the moved APIs from the root crate
- keep compatibility shims only if the boundary audit explicitly approves them
- choose and document the release mechanics for the move:
  - major release
  - temporary compatibility shims
  - deprecation period with migration docs

## Acceptance criteria

- runtime-oriented APIs compile from `evalkit-runtime`
- `src/lib.rs` no longer re-exports the moved APIs
- `examples/prod_eval_daemon.rs` uses `evalkit-runtime`
- any remaining runtime-heavy root dependencies are either removed or explicitly called out as a conscious follow-up
- the semver impact of the move is documented, and the chosen migration path is written down before merge

## Release / migration notes

This issue is not done until the breaking-change story is explicit.

If root exports are being removed from the semver anchor, the PR must say which of these is true:

- this lands in a major release
- temporary compatibility re-exports are kept for one release window
- a deprecation period plus migration guide is shipped before removal

## Test plan

- existing executor behavior tests still pass from the new crate location
- examples compile after the move
- root-crate contract tests added in the follow-up issue can verify the new boundary

## Out of scope

- public rename of `Run` / `Sample`
- platform packaging
- broad scorer work

## Depends on

- `01-audit-root-crate-boundary.md`
