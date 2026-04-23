# Issue: Benchmark the runtime extraction so the production path does not regress

Suggested labels: `performance`, `benchmark`, `runtime`, `medium-priority`

## Summary

Add a small benchmark harness for the runtime path and use it to compare before/after extraction.

The goal is not academic benchmarking. The goal is to catch the boring regressions that happen during boundary cleanup.

## Why this matters

`PullExecutor` and the daemon-style path are already real features. Moving them into a better crate boundary is good. Quietly slowing them down is not.

## Scope

- define one representative benchmark fixture for executor throughput
- record a baseline before the extraction lands
- re-run after extraction
- report throughput and peak resident memory
- if a wasm smoke artifact path exists, also report stripped artifact size and startup footprint before and after the split
- store the benchmark command and interpretation rules in repo docs or bench output comments

## Acceptance criteria

- benchmark can be run locally with a single documented command
- before/after numbers are recorded in the PR or benchmark artifact
- throughput does not regress by more than 10 percent on the fixture
- peak resident memory does not regress by more than 15 percent on the fixture
- if a wasm smoke artifact exists, stripped artifact size and startup footprint do not regress by more than 10 percent from the first passing baseline

## Test plan

- run the documented benchmark before and after the extraction change
- confirm the threshold rule is visible to reviewers

## Out of scope

- broad benchmarking of every scorer or provider
- browser-style performance tooling
- packaging and release automation

## Depends on

- `02-create-evalkit-runtime-and-move-runtime-apis.md`
