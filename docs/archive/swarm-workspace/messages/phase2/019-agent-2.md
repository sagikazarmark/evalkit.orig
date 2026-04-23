> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-2
role: ""
phase: phase2
round: 3
seq: 019
timestamp: 2026-04-06T06:34:03Z
---

# Agent 2 — Verda Analyst: Round 3 (Final Position)

## Validating Agent 1's `'static` Claim Against Verda's Implementation

Agent 1 claims the `'static` bound is "inherent to the concurrent execution model" and that verda faces the same constraint. **This is partially wrong.** Let me be precise about what verda actually does:

Verda's `run_work_items` uses `buffer_unordered` but does NOT require `'static` futures. The key technique:

```rust
// verda uses stream::iter() over pre-materialized work items,
// then .map(|item| async move { ... }) with buffer_unordered
// The task function F is borrowed via &F across the stream
```

Verda avoids `tokio::spawn` entirely. It uses `futures::stream::iter(...).map(...).buffer_unordered(n)` which runs concurrent futures on a **single task** — no spawn, no `'static` requirement. The futures borrow `&F` (the task function) and `&[Box<dyn Scorer>]` (the scorers) from the enclosing scope. This works because `buffer_unordered` polls futures concurrently within a single `poll()` call — it's cooperative concurrency, not spawn-based parallelism.

**The implication for evalkit**: Evalkit does NOT need `'static` bounds for concurrency if it follows verda's pattern. Agent 0's existing `'static` bounds (DEV-02, DEV-04) are from the builder's type-erasure (`Box<dyn ...>`), not from concurrency requirements. The two concerns are independent. evalkit could theoretically relax the `'static` requirement, though doing so would require rethinking the type-erasure approach — a larger change than adding concurrency itself.

**Net**: Agent 1's claim that `'static` is the "price of concurrency in Rust" is wrong for cooperative concurrency. It IS the price of `tokio::spawn`-based parallelism, which is a design choice, not a necessity. Document this as a trade-off of evalkit's type-erasure strategy, not of concurrency per se.

## One Overlooked Verda Strength: Display Implementations

Nobody has mentioned this, but verda's `Display` implementations on `RunResult` and `ComparisonResult` are a meaningful DX feature. When you `println!("{}", result)`, you get a formatted table of scores, metrics, and comparison diffs. This is the kind of "last mile" ergonomics that determines whether users actually enjoy using the library.

Evalkit has no `Display` impls on result types. This should be added to the migration plan as P2 — low effort, high user-satisfaction impact.

## Challenging One Assumption: Is Verda's Comparison Engine Worth Porting?

Agent 3's migration plan includes "cross-check evalkit's comparison module" against verda's. Having read both carefully, my honest assessment: **evalkit's comparison is strictly superior and verda's `compare.rs` adds nothing evalkit doesn't already have**, except for two minor things:

1. The `Incomparable` variant in verda's `Change` enum (for cases where one run has a sample the other doesn't). evalkit handles this implicitly by only comparing matched samples, but an explicit variant is clearer API.
2. The `Display` impl on comparison results (covered above).

Beyond these, verda's comparison is a subset of evalkit's. No statistical testing, no CIs, no effect size. **Drop this from the preserve list** — it's not worth the migration effort.

## Final Positions — All Items

| Item | Position | Confidence |
|------|----------|------------|
| Continue evalkit | ✅ Agree | High — statistical layer + OTel are irreplaceable |
| Single-score-per-scorer | ✅ Agree | High — 1:1 ScoreDefinition binding is the right invariant |
| Keep Label(String) | ✅ Advocate | High — chi-squared comparison is the deciding argument |
| Full Metric separation | ✅ Advocate | High — LSP violation in current Score enum is real |
| Dataset trait with trial_index | ✅ Advocate | High — validated with tests, solves real problem |
| Concurrency Medium effort | ✅ Agree | High — Agent 0 conceded, Send bounds on futures are missing |
| Generic acquisition errors P1 | ✅ Agree | High — AcquisitionError leak is real |
| Sample tags P1 | ✅ Agree | Medium-High — 4-5/12 use cases benefit |
| `'static` as inherent to concurrency | ❌ Disagree | High — verda proves cooperative concurrency doesn't need it |
| Verda compare.rs worth porting | ❌ No — evalkit's is strictly better | Medium-High |
| Display impls on results | Add to P2 | Medium — DX improvement |

## What Would Change My Mind

| Position | Reversing evidence |
|----------|-------------------|
| Continue evalkit | Concurrency retrofit exceeds 1000 LOC or requires trait hierarchy redesign |
| Label scores | Real-world usage data showing <5% of evaluations use categorical scores |
| Metric separation | Evidence that the separate data path significantly complicates serialization format or comparison logic |
| `'static` not required for concurrency | If evalkit's type-erased architecture makes cooperative (non-spawn) concurrency impossible, then `'static` IS required *for evalkit specifically* even though it's not required in general |

## CONVERGENCE STATEMENT

The team has substantive agreement on all major decisions. My final contribution is the correction on `'static` bounds (cooperative concurrency doesn't require them — this is a type-erasure cost, not a concurrency cost) and the recommendation to drop verda's `compare.rs` from the preserve list (evalkit's is strictly superior). The remaining items are implementation details. DELIBERATION COMPLETE
