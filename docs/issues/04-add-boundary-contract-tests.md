# Issue: Add contract tests that prove the kernel boundary change is real

Suggested labels: `tests`, `ci`, `kernel-boundary`, `high-priority`

## Summary

Add build-contract tests that verify the root crate is actually smaller and more portable after the boundary reset.

Behavior tests alone are not enough here. We need tests for the promise, not just the implementation.

## Why this matters

Without contract tests, it is easy to move code around, keep the same dependency baggage, and still tell ourselves the kernel got cleaner.

That is fake progress.

## Scope

- add a CI check for `cargo check -p evalkit --target wasm32-unknown-unknown`
- add compile-level or manifest-level checks that the root crate no longer exports Bucket B APIs
- add a recorded before/after report of the root crate dependency surface
- add facade equivalence tests
- add example-compiles smoke coverage for the recommended quickstart example
- add any necessary test helpers to make boundary checks cheap to run

## Acceptance criteria

- CI reports a passing `wasm32` build for the root crate
- root boundary regressions fail fast in CI
- reviewers can see the root crate dependency surface before and after the boundary change
- facade and underlying kernel path produce equivalent semantics on the fixture cases
- new-user example path is compile-tested

## Test plan

- `cargo test`
- `cargo check -p evalkit --target wasm32-unknown-unknown`
- any manifest/export check added by the implementation

## Out of scope

- runtime extraction itself
- benchmarking
- packaging work

## Depends on

- `02-create-evalkit-runtime-and-move-runtime-apis.md`
- `03-add-semver-safe-happy-path-facade.md`
