---
agent: agent-0
role: ""
phase: phase2
round: 2
seq: 013
timestamp: 2026-04-06T06:23:57Z
---

Good. Now I have concrete evidence for my response.

---

# Agent 0 — Evalkit Analyst: Round 2 Response

## Answering Agent 3's Direct Question

**"Does evalkit's conformance report flag the missing concurrency as a known gap? Is there a design reason it was deferred beyond 'post-MVP'?"**

Yes. The decisions-log (Round 4, lines 339-344) explicitly states:

> **Concurrency: Start sequential.** MVP: samples, trials, scorers all processed sequentially. Simple, debuggable, predictable. `.concurrency(N)` stays in the Run builder API, defaults to 1. Concurrent executor added later — RunResult identical regardless of execution mode. Non-breaking addition: implementation detail behind the same API.

The key insight here: **adding concurrency is structurally trivial**. I verified that `ErasedAcquisition`, `RunExecutor`, `TargetExecutor` all require `Send + Sync`. The `Run` struct holds everything behind `Box<dyn ... + Send + Sync>`. The `execute()` method takes `&self`. Converting the sample loop from sequential `for` to `futures::stream::iter(...).buffer_unordered(self._concurrency)` is approximately a 15-line change. This is genuinely a deferred implementation detail, not a structural limitation.

This contrasts with verda where concurrency is already implemented but the *type system* can't easily be extended to support typed scores or OTel observe mode — those would be structural changes.

**Assessment**: Agent 3's P0 rating on concurrency migration is correct on *priority* but wrong on *effort* — it's Low, not Medium.

## Challenging Agent 3's P0: Multi-Score per Scorer

Agent 3 recommends migrating to `Vec<Score>` returns as P0, calling single-Score-per-scorer "a design mistake." **I disagree**, and Agent 1 independently agrees with me. Here's my specific argument:

### The 1:1 Mapping Enables a Critical Invariant

In evalkit, each scorer has a `definition() → ScoreDefinition` that declares name + direction *before scoring*. This enables:
1. **Pre-execution validation**: The builder checks for duplicate score names at build time
2. **Stats dispatch by type**: The stats layer knows the score type from the definition, not from runtime inspection
3. **Comparison alignment**: When comparing two runs, scores match by definition, not by hoping runtime names are consistent

With `Vec<Score>`, you lose all three. The scorer's `name()` becomes just a label for error messages, and the actual score names are discovered at runtime. This is exactly why verda has 4 error variants for score integrity (`DuplicateScoreName`, `MissingScoreName`, `InvalidScoreValue`, `ConflictingScoreGoal`) — it's catching at runtime what evalkit catches at compile time/build time.

### Agent 3's Motivating Example is Wrong

> "A classification scorer needs to emit precision, recall, F1 in one pass."

Precision, recall, and F1 are **aggregate metrics computed over a dataset**, not per-sample scores. A per-sample classification scorer emits one thing: `Binary(prediction == reference)` or `Label(predicted_class)`. Precision/recall/F1 are derived from the aggregation of those per-sample scores across the dataset. Evalkit's stats layer is exactly where this computation belongs.

If the argument is "but I want accuracy + latency from one scorer" — latency is a measurement, not a judgment. This is the Score/Metric separation discussion, not a multi-score-per-scorer argument.

### What Would Change My Mind

If someone demonstrates a real use case where a single per-sample evaluation *genuinely produces multiple independent quality judgments* that can't be decomposed into separate scorers without re-running expensive computation. The ScorerSet with shared mapper already handles the "expensive shared transform" pattern. I haven't seen a case that ScorerSet can't handle.

**My recommendation**: Keep single-Score-per-scorer. It's not a P0 migration — it's the correct design.

## Extending: Score::Metric Separation

Agent 2 and Agent 3 both argue Score::Metric should be separated from Score. I **partially agree** but want to add nuance.

The real problem isn't that Metric is in the Score enum — it's that `Score::Metric` has a `name` field while other Score variants don't. This asymmetry means:
- `Numeric(f64)` gets its name from `ScoreDefinition`
- `Metric { name, value, unit }` carries its own name

This is genuinely confusing. There are two clean solutions:

1. **Remove Score::Metric entirely** (Agent 2/3's position): Separate type, separate flow
2. **Remove the name from Score::Metric**, use ScoreDefinition consistently: `Score::Metric { value, unit }` where the name comes from ScoreDefinition like every other variant

Option 2 preserves evalkit's type-appropriate aggregation (unit-aware stats for metrics) without creating a separate data path. It's lower-effort and more consistent. I'd recommend this over a full separation.

## Extending: Dataset Trait

I agree with Agent 2 and Agent 3 that verda's `Dataset` trait with `sample(index, trial_index)` is the superior design. Let me add a concrete argument:

Evalkit's `Dataset` struct forces `samples: Vec<Sample<I, R>>` — all samples materialized upfront. For the brain dump agent use case (UC-5), this means if you have 10,000 classified thoughts, you load all 10,000 into memory before running. A trait-based approach allows lazy loading, streaming from a database, or — critically — **deterministic augmentation per trial** (e.g., adding noise to test robustness).

The migration is clean: replace `pub struct Dataset<I, R>` with `pub trait Dataset<I, R>: Send + Sync`, add `impl<I, R> Dataset<I, R> for Vec<Sample<I, R>>` for backward compatibility. Evalkit's `metadata: HashMap<String, Value>` on the current Dataset struct would need to move to a separate concern or be added to the trait.

## New Finding: AcquisitionError Leak is Worse Than I Thought

On closer review, the `AcquisitionError` variants aren't just a mental-model issue — they affect error handling in `execute_trial` (line 162):

```rust
Err(err) => acquisition_failure_scores(&self.definitions, err),
```

When acquisition fails, evalkit generates `ScorerError::AcquisitionFailed` for *every* scorer definition. The error message includes the AcquisitionError display. If a non-AI user sees `TraceNotFound { correlation_id: "...", sample_id: "..." }` in their compiler optimization eval, it's actively confusing.

Agent 3's recommendation to "allow `Fn(I) -> Result<O, E>` where E: Error" (P2) should be elevated. The acquisition trait should accept user-defined error types, with evalkit's OTel-specific variants available only behind the feature gate.

## Correcting My Round 1 Deviation Count

I stated "5, not 4" deviations in Round 1. After re-examining: the spec conformance doc lists DEV-01 through DEV-05, plus D-01 and D-02 as "known omissions." My Round 1 correction was accurate — there are 5 deviations + 2 omissions. The task description's "4 documented deviations" appears to be an approximation. Minor point, but I want to be precise.

## Summary of Where I Stand

**Agreements with the team:**
- Continue evalkit (the statistical layer and OTel support are non-replicable advantages)
- Migrate concurrency from verda (trivially feasible — Low effort, not Medium)
- Migrate Dataset trait pattern from verda (clean migration path exists)
- Migrate sample tags from verda (Low effort, high value for sliced analysis)

**Disagreements:**
- **Vec<Score> migration is NOT P0 and is NOT correct.** Single-Score-per-scorer + ScorerSet is the better design. The build-time validation it enables is worth more than the minor ergonomic gain of multi-score returns.
- **Score::Metric should be fixed, not removed.** Drop the `name` field from the variant; let ScoreDefinition handle naming consistently.
- **Generic acquisition errors** should be P1, not P2 — the AcquisitionError leak is more problematic than I initially assessed.

**What would change my mind on the core recommendation**: If verda's codebase proved significantly easier to extend (suggesting evalkit's spec-driven complexity is a net negative for maintenance), I'd reconsider. But the evidence from the execution model (all `Send + Sync`, clean separation) suggests evalkit is equally extensible *and* starts from a richer baseline.

---

I believe the team is converging productively. The major open question is the single-vs-multi-score-per-scorer decision, which is consequential enough to deserve explicit resolution. I'm ready for Agent 3's synthesis once Agents 1 and 2 weigh in on Round 2.
