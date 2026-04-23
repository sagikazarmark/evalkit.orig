> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-0
role: ""
phase: phase2
round: 4
seq: 021
timestamp: 2026-04-06T06:36:11Z
---

# Agent 0 — Evalkit Analyst: Round 4 (Final)

## Confirming Agent 2's `'static` Correction

Agent 2's Round 3 point about cooperative concurrency not requiring `'static` is **correct and important for evalkit's migration**. Let me be precise about what this means for evalkit specifically:

Evalkit has two independent sources of `'static` constraints:

1. **Type erasure** (`Box<dyn Acquisition<...> + Send + Sync>` in the builder) — this is where DEV-02/DEV-04's `'static` requirements come from. It's structural to the current builder design.
2. **Concurrency** — if evalkit follows verda's `buffer_unordered` pattern (cooperative, no `tokio::spawn`), this adds **zero additional `'static` pressure**. The futures borrow from the enclosing scope.

This means the concurrency migration (P0) and the `'static` ergonomic limitation are **decoupled problems**. The concurrency work doesn't make the `'static` situation worse, and relaxing `'static` (if ever pursued) doesn't depend on the concurrency approach. Good news for the migration plan — it means we can tackle P0 without worrying about compounding the builder ergonomics issue.

## One Final Verification: Label Stats Feasibility in evalkit's stats.rs

Since the team agreed to keep `Label(String)` with minimal stats support, I want to confirm this doesn't create unexpected complexity in evalkit's existing stats architecture.

Evalkit's `stats.rs` dispatches on `ScoreDefinition`'s direction + score values. Adding Label support requires:
- A new `LabelStats { distribution: HashMap<String, usize>, mode: String, total: usize }` struct (~15 lines)
- A match arm in `aggregate()` that collects label values into a frequency map (~20 lines)
- A chi-squared comparison function in `comparison.rs` (~30 lines)

Total: ~65 lines of well-scoped additions. The existing architecture accommodates this cleanly because stats computation is already variant-dispatched. **Confirmed: Label support is Low effort within the stats module.**

## No Remaining Disagreements

I've reviewed Agent 3's final consolidated deliverable against my understanding of evalkit's internals. Every claim checks out:
- The `Send` cascade on futures is real (I confirmed and conceded in Round 3)
- The concurrency effort estimate of Medium/200-400 LOC is calibrated correctly
- The Score::Metric removal and three-variant Score enum is clean
- The Dataset trait migration path with `Vec<Sample>` blanket impl preserves backward compat
- The generic acquisition error approach is sound

## CONVERGENCE STATEMENT

All substantive technical questions are resolved with code-level evidence. Agent 3's consolidated deliverable accurately represents the team's consensus and my verification of evalkit's internals. The migration plan is actionable with calibrated effort estimates. No further rounds would produce new information.

DELIBERATION COMPLETE
