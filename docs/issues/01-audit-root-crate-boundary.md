# Issue: Audit the `evalkit` root crate boundary

Suggested labels: `architecture`, `kernel`, `docs`, `high-priority`

## Summary

Audit every root-crate export and classify it as kernel, optional/runtime, or explicit-decision-required.

The current root crate is the semver anchor, but it exports more than a minimal kernel should. That means we are freezing too much surface by default.

## Why this matters

This is the decision issue.

If we skip it, every later extraction PR will argue about scope in the middle of code changes. That is the worst time to make boundary decisions.

## Scope

- audit all public exports in `src/lib.rs`
- classify each export into one of three buckets:
  - keep in kernel
  - move to optional/runtime crate
  - explicit follow-up decision required
- make explicit decisions for the current Bucket C candidates instead of leaving them fuzzy:
  - `Run`
  - `read_jsonl` / `write_jsonl`
  - `ConversationSample`
  - `TrajectorySample`
- update `docs/stability.md` so it reflects the intended stable surface after the reset
- add or update one ASCII diagram showing the crate boundary
- record any root APIs that are intentionally left as exceptions

## Acceptance criteria

- every root export in `src/lib.rs` is classified
- the disposition of `Run`, `read_jsonl` / `write_jsonl`, `ConversationSample`, and `TrajectorySample` is explicitly recorded
- `docs/stability.md` no longer implies that accidental root exports are stable forever
- the boundary decision is written in repo, not only in chat
- the output is concrete enough to unblock extraction PRs without reopening first-principles debate

## Test plan

- no runtime tests required
- reviewer can map each `pub use` in `src/lib.rs` to a documented bucket with no ambiguity

## Out of scope

- moving code
- adding a new facade
- benchmarking
- packaging or distribution planning

## Depends on

- none
