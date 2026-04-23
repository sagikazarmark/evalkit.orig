# evalkit Kernel Boundary Plan

> **Supersedes:** the execution-planning portions of `docs/verda-competitive-analysis.md`
>
> **Amends:** `docs/ROADMAP.md` and `docs/gap-analysis.md` for the specific question of current kernel-boundary priority, sequencing, and verification
>
> **Does not replace:** the full long-range roadmap in `docs/ROADMAP.md` or the current-state inventory in `docs/gap-analysis.md`

This document replaces the stale planning parts of `docs/verda-competitive-analysis.md`.

That file is still useful as market research. It is not a reliable execution plan. The repo has already moved further than the document admits, and the root crate boundary is now the main architectural problem.

## Thesis

`evalkit` should be the easiest Rust-native eval kernel to embed.

Not the easiest eval platform to self-host. Not the broadest scorer catalog. Not a kitchen-sink root crate that happens to contain a kernel somewhere inside it.

The product story only works if the crate boundary matches the pitch.

## The actual problem

Today the semver anchor is too wide.

`src/lib.rs` re-exports batch execution, online execution, samplers, sources, sinks, sharding, scrubbers, JSONL helpers, schema helpers, and sample-shape types from the root crate. `docs/stability.md` says those root exports are stable. That means we are freezing a lot more than a minimal kernel.

This is the whole issue.

Removing `tokio` alone is not enough. The root crate needs an explicit boundary reset.

## Strategic call

For now, keep the name `evalkit` in repo-facing docs.

The category claim is still:

- small Rust-native eval kernel
- optional runtime and platform surfaces
- provider-neutral scoring and acquisition seams
- portable enough to be credible for constrained runtimes

The repo can keep server, CLI, exporter, provider, and OTel crates. Those are useful. They just cannot define what the root crate is.

## What stays true

- `evalkit-server` can exist without turning `evalkit` into a platform-first product.
- `evalkit-otel` remains a differentiator.
- `evalkit-scorers-llm` and the scorer crates remain good optional surfaces.
- The current roadmap still contains useful direction, but not all of it belongs in the root crate.

## Boundary decision

The root crate must be re-audited and split into three buckets.

### Bucket A: definitely kernel

These belong in `evalkit` unless the audit finds a strong reason otherwise:

- score model and score definitions
- `Scorer`, `ScorerContext`, `ScorerSet`, scorer composition
- core sample and dataset types
- run results, comparison, stats, schema types
- the smallest stable batch entrypoint, if it can remain runtime-light

### Bucket B: definitely optional/runtime

These should leave the root crate:

- `Executor`, `PullExecutor`
- samplers
- sources and sinks
- sharding
- scrubbers
- partial-stream and checkpoint scoring helpers
- file tailing and other runtime-oriented helpers

### Bucket C: explicit decision required

Do not inherit these into the kernel by accident. Decide them on purpose.

- `Run` if it still forces runtime-heavy internals
- JSONL read/write helpers
- conversation / trajectory sample shapes
- any helper whose main value is operational rather than semantic

## Desired crate shape

```text
                           +-----------------------+
                           |      evalkit          |
                           |  semver anchor        |
                           |-----------------------|
                           | Score / Scorer /      |
                           | Dataset / Sample /    |
                           | RunResult / Compare / |
                           | Stats / Schema /      |
                           | smallest stable front |
                           +-----------+-----------+
                                       |
                +----------------------+----------------------+
                |                      |                      |
                v                      v                      v
     +------------------+   +-------------------+   +----------------------+
     | evalkit-runtime  |   | evalkit-providers |   |  evalkit-scorers-*   |
     |------------------|   |-------------------|   |----------------------|
     | Executor         |   | HTTP/subprocess   |   | text / llm / rag /   |
     | PullExecutor     |   | acquisition impls |   | embed / redteam      |
     | Samplers         |   | plugin protocol   |   | optional scorer packs|
     | Sources / Sinks  |   +-------------------+   +----------------------+
     | Sharding         |
     | Scrubbers        |
     | Stream helpers   |
     +--------+---------+
              |
      +-------+--------+------------------+
      |                |                  |
      v                v                  v
+-------------+  +-------------+  +------------------+
| evalkit-cli |  | evalkit-otel|  |  evalkit-server  |
| local DX    |  | traces/emit |  | optional app     |
+-------------+  +-------------+  +------------------+
```

## What we are not doing

- Not rewriting the product around a hosted control plane.
- Not turning the root crate into a registry for every optional surface.
- Not doing a public rename of `Run` / `Sample` into a new vocabulary in the same change.
- Not broadening the scorer catalog just because Python competitors have bigger lists.
- Not pulling packaging and distribution planning into this document yet. That work is real, but deferred.

## The next slice

The work should happen in this order.

1. Audit the root crate boundary.
2. Introduce `evalkit-runtime` and move the clearly runtime-oriented APIs.
3. Add a semver-safe happy-path facade on top of the surviving kernel API.
4. Add contract tests that prove the boundary change is real.
5. Add a small executor regression benchmark so the runtime move does not quietly slow the production path.

## Verification

The refactor is only done if all of these are true.

### Build-contract checks

- `cargo test` still passes for the preserved kernel behavior.
- `cargo check -p evalkit --target wasm32-unknown-unknown` passes.
- the root crate no longer re-exports the APIs classified into Bucket B.
- any remaining runtime-heavy root exports are explicitly documented as a conscious exception.

### Facade checks

- the new happy-path facade produces the same score definitions, trial counts, and run-result semantics as the underlying kernel path it wraps.
- the example shown to users compiles and runs in CI.

### Performance checks

- record a baseline benchmark before extraction.
- fail the work if executor throughput regresses by more than 10 percent on the benchmark fixture.
- fail the work if peak resident memory grows by more than 15 percent on the same fixture.
- record the post-split dependency surface for `evalkit` and keep it flat or lower than the pre-split baseline.

### Portability checks

- keep a reported `wasm32` compile result in CI.
- if a wasm smoke artifact is emitted, fail the work if stripped artifact size or startup footprint regresses by more than 10 percent from the first passing baseline.

## Why this is the right-sized move

This is not a rewrite.

Most of the runtime capability already exists. The server exists. The OTel crate exists. The scorer crates exist. The job is to stop pretending those optional surfaces are the same thing as the kernel.

That is smaller than a redesign and more honest than another round of aspirational docs.

## Issue set

Execution issues live in `docs/issues/`:

- `docs/issues/01-audit-root-crate-boundary.md`
- `docs/issues/02-create-evalkit-runtime-and-move-runtime-apis.md`
- `docs/issues/03-add-semver-safe-happy-path-facade.md`
- `docs/issues/04-add-boundary-contract-tests.md`
- `docs/issues/05-benchmark-runtime-extraction.md`

## Do Not Forget

- The real job is shrinking the semver-critical root crate, not just removing `tokio`.
- `docs/stability.md` is part of the implementation surface. Any root-export move is also a release-contract decision.
- Do `01` before `02`. If the audit is skipped, code review will turn into a scope argument.
- The slippery decisions are still `Run`, JSONL helpers, and the conversation / trajectory sample shapes. Those calls will decide whether the boundary cleanup is real or cosmetic.
- `wasm32` compile success is necessary, not sufficient. Dependency surface and portability budgets matter too.
- The additive facade must stay semantically equivalent to the kernel path, or the project will accidentally split into a demo API and a real API.
- Packaging and naming were deferred, not solved. They should not be forgotten just because they are not in the kernel-boundary milestone.
