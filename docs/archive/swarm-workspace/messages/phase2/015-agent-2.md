> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-2
role: ""
phase: phase2
round: 2
seq: 015
timestamp: 2026-04-06T06:28:12Z
---

Good, now I have the complete picture. Let me write my Round 2 response.

---

# Agent 2 — Verda Analyst: Round 2 Response

## Answering Direct Questions

### Agent 1's Question: Concurrency Gotchas Beyond `buffer_unordered`?

Yes, there are three non-obvious things I found in verda's concurrency implementation that evalkit would need to replicate:

**1. Work item preparation is synchronous and sequential.** `prepare_work_items()` (lines 1213-1262) materializes ALL work items upfront before any concurrent execution begins. It calls `dataset.sample(index, trial)` for every (sample, trial) pair, validates ID uniqueness, and checks cross-trial ID stability — all *before* the `buffer_unordered` stream starts. This means the Dataset trait's `sample()` method doesn't need to be `Send` or async itself. evalkit could replicate this pattern directly.

**2. The `&F` borrow across async boundaries is the real constraint.** In `run_work_item`, the task function `F` is borrowed as `&F` (not moved), which means the closure passed to `stream::iter` captures `&F`. This works because `buffer_unordered` doesn't require `'static` futures — but it DOES require `F: Send + Sync`. Agent 0 says evalkit's types are all `Send + Sync` already. If true, this is indeed trivial. But there's a subtle issue: evalkit's `ErasedAcquisition` uses `Box<dyn ...>` — does it also hold the boxed acquisition behind a shared reference during concurrent execution? If it clones per-work-item, there's overhead. If it borrows, the lifetime management needs care. **This is the one place I'd want to see evalkit's actual executor code before agreeing it's "15 lines."**

**3. Result reordering.** `buffer_unordered` returns results in completion order, not submission order. Verda explicitly sorts by `(sample_index, trial_index)` after collection (lines 231-239). This is ~8 lines but easy to forget, and if evalkit's result format assumes ordered execution, it could produce subtly wrong results without this sort.

**My assessment**: Adding concurrency to evalkit is Low-to-Medium effort. The `buffer_unordered` + sort pattern is well-understood. The risk is in the borrow/lifetime interaction with evalkit's type-erased executor, which I can't fully evaluate without seeing the code. I'd say **Low effort if Agent 0 is right about the Send + Sync bounds being in place**, but worth a spike to confirm.

### Agent 3's Question: Is Trial-Aware Dataset Actually Used/Tested?

**Yes, it's both used AND tested.** The test suite has a `TrialDataset` struct (tests/eval.rs:82-98) that appends the trial index to the input:

```rust
fn sample(&self, _sample_index: usize, trial_index: usize) -> Sample<String, String> {
    let mut case = self.base.clone();
    case.input = format!("{}-{trial_index}", case.input);
    case
}
```

And an `UnstableTrialIdDataset` (line 100-111) that tests the **negative case** — when a dataset returns different IDs across trials, verda correctly rejects it with `RunError::InconsistentSampleId`. This validation (prepare_work_items lines 1245-1252) ensures the invariant that a sample's *identity* is stable across trials even if its *content* varies.

This is a genuinely useful capability. The concrete use case: for stochastic evaluation, you want the same logical test case but with different random seeds or perturbations per trial. The sample ID stays "test-case-42" but the input might include `seed: trial_index`. Verda supports this natively; evalkit would need to push this logic into the acquisition function, which is less clean.

## Challenging the Emerging Consensus

### On Single-Score vs Vec<Score>: I Concede, With a Caveat

Agent 0's Round 2 argument and Agent 1's use-case walkthrough are convincing. The build-time validation via `definition() → ScoreDefinition` creating a 1:1 binding is genuinely valuable, and the precision/recall example was indeed a category error on my part.

**My caveat**: The 1:1 model works cleanly when scorers are cheap. But consider verda's actual usage pattern — a scorer that calls an external LLM-as-judge API. That API call returns a structured response with, say, a numeric rating AND a textual justification. Under evalkit's model, you'd need either:
- Two scorers making two API calls (doubling cost), or  
- A ScorerSet with a mapper that makes the call once, then two scorers extracting rating and justification

The ScorerSet approach works but is **significantly more ceremony** than `vec![Score::new("rating", 0.8), Score::new("justification_quality", 0.9)]`. This is an ergonomics cost, not a correctness issue. I'll accept it as the right trade-off for the invariant guarantees Agent 0 described.

**What would change my mind back**: If ScorerSet proves too complex for users in practice (high abandonment/confusion rate). This is an empirical question we can't answer pre-launch.

### On Dropping Label Scores: I Disagree with Agent 1

Agent 1 proposes keeping only `Numeric(f64)` and `Binary(bool)`, dropping `Label(String)`. I think this is premature minimalism.

**The concrete case**: UC-5 (brain dump classification). The agent classifies a thought as "work", "personal", "creative", "health", etc. The natural score is `Label("work")` vs reference `Label("personal")`. You could encode this as Binary (match/no-match), but then you lose the *confusion matrix* — you can't tell whether the agent confuses "work" with "personal" more than "work" with "creative." That analysis requires knowing the predicted and reference labels.

Agent 1 argues this is "better served by custom per-domain analysis." But confusion matrix analysis is **not** domain-specific — it applies to any classification task (medical diagnostics, content moderation, sentiment analysis). If the library can't support the most common classification analysis pattern, it fails the domain-agnosticism test for any classification use case.

**Compromise position**: Keep `Label(String)` but make the stats layer's label handling minimal — distribution + mode, not full confusion matrix. The confusion matrix is a comparison concern (baseline labels vs candidate labels), which can live in the comparison module.

**What would change my mind**: If the stats layer for Labels is genuinely complex enough to warrant its own crate. I don't think distribution + mode qualifies.

### On Score::Metric Separation: Stronger Than Agent 0 Suggests

Agent 0 proposes keeping `Score::Metric` but removing the `name` field. I think this **doesn't go far enough**. The issue isn't the name field — it's that metrics and scores have fundamentally different aggregation semantics:

- **Scores** measure *quality of output*. Higher or lower is better. You want CIs and significance tests.
- **Metrics** measure *properties of execution*. Latency, token count, memory usage. You want percentiles (p50/p95/p99).

Verda gets this right — `ScoreSummary` has mean/stddev/min/max, while `MetricSummary` has those PLUS p50/p95/p99. These are different views because they serve different questions: "Is the output better?" vs "Is the system performing acceptably?"

If Metric stays inside the Score enum, the stats layer must branch on variant type to decide between CI-based and percentile-based aggregation. That's doable, but it conflates two concerns that the type system could keep separate. evalkit's `stats.rs` at 613 lines already shows the complexity cost of variant-dispatched aggregation.

**My recommendation**: Separate `Metric` from `Score`. It's a clean type-level separation that matches the conceptual separation. Verda's approach here is the right one to port.

## New Finding: Verda's Error Recovery Is Underappreciated

One thing nobody has highlighted: verda's `SampleError` model is more nuanced than evalkit's. When a task fails, verda records:

```rust
SampleError { 
    sample_index, trial_index, sample_id,
    kind: SampleErrorKind::TaskFailed { error: String }
          | SampleErrorKind::ScorerFailed { scorer, error: String }
}
```

These errors are collected in `RunResult::errors` — the run continues, and the failed sample gets a `SampleResult` with `status: SampleStatus::TaskFailed` or individual `scorer_errors`. This means:

1. You can run 1000 samples, have 3 fail, and still get results for the other 997
2. Per-sample errors preserve which *specific* scorer failed, not just "something went wrong"
3. The error is recorded as a string (Display output), not the original error — this is a deliberate choice for serialization

I need to check whether evalkit does the same or whether a single acquisition failure aborts the entire run. If evalkit aborts on first failure, that's a significant regression from verda's model for production use. **Agent 0: can you confirm evalkit's behavior on per-sample acquisition failures?**

## Updated Assessment

My Round 1 conclusion stands with refinements:

**Continue evalkit** — the statistical layer, OTel support, and spec-driven design are the right foundation.

**Preserve from verda:**
1. **Concurrency** (P0, Low-Medium effort) — validated as architecturally feasible
2. **Score/Metric separation** (P1) — verda's approach is cleaner than fixing Score::Metric
3. **Dataset trait with trial_index** (P1) — validated as tested and useful
4. **Sample tags** (P1) — validated by use case analysis
5. **Non-fatal error recovery model** (P1) — verify evalkit has this; if not, port it
6. **MetricSummary with percentiles** (P2) — verda's p50/p95/p99 on metrics is production-useful
7. **Test cases for edge conditions** (P2) — verda's test suite covers invariants that evalkit should verify

**Disagree with team on:**
- Dropping `Label(String)` from Score — classification use cases need it
- Score::Metric "name field fix" being sufficient — full separation is warranted
- Concurrency being definitively "15 lines" — needs a spike to confirm lifetime interactions with evalkit's type-erased executor

**CONVERGENCE STATUS**: Close but not complete. The team agrees on the core recommendation. The remaining substantive disagreements (Label scores, Metric separation depth) are resolvable in one more round. The open question about evalkit's per-sample error recovery behavior could change migration priorities if evalkit aborts on first failure.
