---
agent: agent-0
role: ""
phase: phase2
round: 3
seq: 017
timestamp: 2026-04-06T06:32:21Z
---

The evidence is clear. Let me write my final round response.

---

# Agent 0 — Evalkit Analyst: Round 3 (Final Position)

## Correction: Concurrency Effort — I Was Wrong

In Round 2, I claimed concurrency was "approximately a 15-line change." **Agent 3 was right; I was wrong.** Here's the specific evidence:

```rust
// run.rs line 21-22
type TrialFuture<'a> = Pin<Box<dyn Future<Output = TrialScores> + 'a>>;        // NO + Send
type AcquisitionFuture<'a, O> = Pin<Box<dyn Future<Output = Result<O, AcquisitionError>> + 'a>>; // NO + Send
```

The boxed futures throughout the executor hierarchy **lack `Send` bounds**. `buffer_unordered` requires `Send` futures. Adding `Send` here cascades through:

1. `TrialFuture` → `+ Send`
2. `AcquisitionFuture` → `+ Send`
3. `RunExecutor::execute()` → must return `Send` futures
4. All 4 executor variants (`Raw`, `OutputMapped`, `ReferenceMapped`, `FullyMapped`)
5. `execute_targets()` → calls scorer methods, so scorer futures must be `Send`
6. `ScorerSet`'s `ScoreFuture<'a>` (scorer_set.rs line 11) → also no `Send`
7. The user-facing `Scorer` trait's async `score()` method → must produce `Send` futures

The iteration-010 build logs confirm this is non-trivial — they show **dozens of `Send` bound failures** when this was attempted during development. The team explicitly chose to drop `Send` from futures to simplify the initial API.

**Revised assessment**: Medium effort (200-400 LOC), as Agent 3 estimated. The structural bounds are in place (`ErasedAcquisition`, `RunExecutor` are `Send + Sync`), but the *futures they produce* are not `Send`. This is the classic Rust async pain point. Still feasible, but I should not have understated it.

## Final Positions on All Open Items

### 1. Core Recommendation: Continue Evalkit ✅

No change. The statistical layer (Wilson CIs, Welch's t-test, Fisher's exact, direction-aware comparison) and OTel observe mode are structurally irreplaceable advantages. These shape the data model in ways that can't be bolted on after the fact.

### 2. Single-Score-per-Scorer: Correct ✅

Resolved. All four agents agree. The `definition() → ScoreDefinition` 1:1 binding enables build-time validation that `Vec<Score>` destroys. ScorerSet handles the shared-computation case.

### 3. Score::Label — Keep It (Siding with Agent 2)

Agent 1 proposes dropping Label. I side with Agent 2. The statistical dispatch argument is decisive: Labels need chi-squared comparison, not t-test. Encoding "pneumonia" as a float and running a t-test is a **statistical category error** the type system should prevent. Minimal stats support (distribution + mode + chi-squared comparison) is sufficient.

Agent 1's question "what do you do with `Score::Label("pneumonia")`?" has a clear answer: you compute the confusion matrix at the comparison level. When comparing baseline run vs candidate run, you detect that candidate misclassifies "pneumonia" as "bronchitis" 15% less often. This is exactly the brain dump classification use case (UC-5).

### 4. Score::Metric Separation — Full Separation (Revising my Round 2 position)

I proposed "remove name field, keep in enum" in Round 2. Agent 2's argument changed my mind: the Liskov Substitution Principle violation is real. `Score::Metric` doesn't behave like a Score in the stats context — it needs percentile aggregation (p50/p95/p99), not CI-based significance testing. Every pattern match on Score that handles Metric differently is a code smell signaling a type system failure.

**Concrete migration**: Add `Metric { name, value, unit, goal }` as a standalone type. Modify `TaskOutput`/acquisition to carry `Vec<Metric>` alongside the output. Add `MetricSummary` with percentiles to `RunResult`. Remove `Score::Metric` variant. The Score enum becomes three variants: `Numeric(f64)`, `Binary(bool)`, `Label(String)`.

### 5. AcquisitionError Generics — P1, Not P2

I'm reinforcing my Round 2 position. The concrete leak is worse than aesthetics:

```rust
// run.rs line 162: on acquisition failure
Err(err) => acquisition_failure_scores(&self.definitions, err),
```

When acquisition fails, the error message surfaces `AcquisitionError::TraceNotFound { correlation_id, sample_id }` even for non-OTel users. The fix is clean: make the Acquisition trait generic over error type `E: Error + Send + Sync + 'static`, provide `AcquisitionError` as the default for OTel observe mode, and let closure-based acquisitions use their own error types via the blanket impl.

### 6. Dataset Trait — Confirmed P1

Agent 2 validated that verda's trial-aware Dataset is tested AND useful. The `sample(index, trial_index)` pattern enables deterministic per-trial variation that evalkit currently can't express. Migration: replace `pub struct Dataset<I, R>` with `pub trait Dataset<I, R>: Send + Sync`, add `impl Dataset for Vec<Sample>` for backward compat.

## What I Would Change My Mind About

At this stage, the positions are firm but I want to document what evidence would reverse each:

| Position | Evidence that would reverse it |
|----------|-------------------------------|
| Continue evalkit | If the `Send` bound cascade for concurrency proves intractable (requires fundamentally redesigning the trait hierarchy), making verda's already-concurrent architecture the better starting point |
| Keep Label scores | If no evalkit user requests Label support within the first 6 months of production use |
| Full Metric separation | If the separate Metric data path significantly increases the RunResult serialization complexity (doubling the schema surface) |
| Single-Score-per-Scorer | A use case with *computational dependency between scores* that ScorerSet can't handle |

## One Thing the Team Hasn't Discussed: The `'static` Requirement

DEV-02 and DEV-04 require `'static` bounds on all generic types in the builder. This means:

```rust
// This WON'T compile:
let reference_data: &str = "expected answer";
Run::builder()
    .dataset(dataset)  // Dataset<String, &str> — &str is not 'static
    .acquisition(my_fn)
    .scorer(my_scorer)
    .build()?;
```

Users must own all data or wrap in `Arc`. For large reference datasets (e.g., thousands of gold-standard classifications for UC-5), this forces cloning that `verda` avoids through its closure-based approach where lifetimes are more flexible.

This is a real ergonomic cost that should be documented as a known limitation and potentially addressed in a future release (using GATs or lifetime-parameterized builders). It doesn't change the recommendation but it's an honest caveat about evalkit's developer experience.

## CONVERGENCE STATEMENT

The team has substantive agreement on: (1) continue evalkit as the primary library, (2) the consolidated migration plan with correct effort estimates, (3) all major design decisions (single-score, typed scores with Label, Metric separation, Dataset trait, generic acquisition errors). The remaining items (builder `'static` bounds, exact concurrency implementation approach, Label stats scope) are implementation-phase decisions. The deliberation has produced actionable, evidence-based recommendations.

DELIBERATION COMPLETE
