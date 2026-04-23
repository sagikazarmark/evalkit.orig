# Issue: Add a semver-safe happy-path facade for first-use DX

Suggested labels: `api`, `dx`, `kernel`, `medium-priority`

## Summary

Add a small additive facade that makes `evalkit` easier to start with, without renaming or deprecating the stable kernel API.

This should feel closer to "data + task + scorers" while still compiling down to the existing kernel model.

## Why this matters

The current API is solid, but the first five minutes still ask the user to understand more structure than the best competitors do.

We want a cleaner front door, not a semver bonfire.

## Scope

- design one additive happy-path entrypoint
- keep `Run`, `Sample`, and the current stable kernel surface intact
- add one user-facing example that uses the new facade
- document when to use the facade vs the lower-level kernel API

## Constraints

- no breaking rename of existing public types
- no duplicate execution engine hidden behind the facade
- no widening of the root crate beyond the audited kernel boundary

## Acceptance criteria

- a new user can define a small eval with less ceremony than the current `Run::builder()` path
- the facade is additive, not a breaking replacement
- docs and examples show the facade as the recommended quickstart path
- facade equivalence is defined concretely on fixture cases:
  - same score definitions
  - same trial counts
  - same `RunMetadata` semantics
  - same result shape and score outcomes

## Test plan

- add equivalence tests comparing facade output to the underlying kernel path
- add an example-compiles smoke test for the facade example

## Out of scope

- full API rename to `Eval` / `Case` / `Task`
- runtime extraction
- scorer-catalog expansion

## Depends on

- `01-audit-root-crate-boundary.md`
