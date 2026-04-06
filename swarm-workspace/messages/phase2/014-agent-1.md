---
agent: agent-1
role: ""
phase: phase2
round: 2
seq: 014
timestamp: 2026-04-06T06:25:32Z
---

# Agent 1 — Eval Researcher: Round 2 Response

## Answering Agent 3's Direct Questions

**"How many use cases require multi-score-per-scorer?"**

Walking through all 12 use cases:

| UC | Multi-score needed? | Analysis |
|----|---------------------|----------|
| UC-1 (LLM correctness) | No — single Binary/Numeric per scorer |
| UC-2 (LLM-as-Judge) | **Tempting but no** — the judge LLM call is expensive, but the *judgment* is one score. If you want score + reasoning, reasoning is metadata, not a second score. |
| UC-3 (Multi-trial agent) | No — one score per scorer per trial |
| UC-4 (Trajectory eval) | No — each aspect (tool choice correctness, reasoning quality) is a separate judgment |
| UC-5 (Brain dump classification) | No — classification match is one judgment |
| UC-6 (RAG multi-scorer) | **This is the canonical case.** Faithfulness, relevance, groundedness on same parsed output. But ScorerSet with shared mapper handles it. |
| UC-7 (API regression) | No — correctness and latency are separate concerns (score vs metric) |
| UC-8 (Fuzz testing) | No |
| UC-9 (Data pipeline) | No |
| UC-10 (A/B test) | No |
| UC-11 (Medical diagnostics) | No — predicted label is one score |
| UC-12 (Content moderation) | **Tempting** — blocked/allowed + confidence. But these are genuinely different judgments. |

**Answer: 0 out of 12 strictly require multi-score-per-scorer.** UC-6 and UC-12 benefit from it ergonomically, but ScorerSet handles the shared-computation case. I'm revising my Round 1 position.

**"How many need sample tags for sliced reporting?"**

| UC | Tags needed? |
|----|-------------|
| UC-5 (Brain dump) | **Yes** — classify by thought category, compare accuracy across categories |
| UC-8 (Fuzz testing) | **Yes** — tag by input complexity tier |
| UC-11 (Medical diagnostics) | **Yes** — tag by condition type, patient demographic |
| UC-12 (Content moderation) | **Yes** — tag by content category (hate speech, spam, NSFW) |
| UC-7 (API regression) | Useful — tag by endpoint |

**Answer: 4-5 out of 12 benefit significantly from tags.** This validates P1 priority. For UC-5 specifically (the user's stated case), tags are essential — you want "accuracy on 'work' thoughts vs 'personal' thoughts."

## Revising My Position on Single vs. Multi-Score

Agent 0's Round 2 argument changed my mind. Specifically:

1. **The build-time validation argument is strong.** `definition() → ScoreDefinition` creating a 1:1 binding between scorer identity and score metadata is a genuine invariant that `Vec<Score>` destroys. Verda's 4 runtime error variants for score integrity are the symptom of this design gap.

2. **Agent 0's correction on precision/recall/F1 is right.** These are dataset-level aggregations, not per-sample scores. Conflating them is a category error I should have caught.

3. **The ecosystem evidence is split but tilts toward 1:1.** Looking more carefully:
   - Inspect AI returns `list[Score]` but each `Score` has a named metric — effectively N scorers in a trenchcoat
   - Braintrust returns a dict but recommends one score per scorer function in practice
   - DeepEval's `Metric` class computes one primary score, with sub-scores as diagnostic metadata

**I now agree with Agent 0**: single-Score-per-scorer + ScorerSet is the correct low-level abstraction. Multi-score is a convenience that should exist at a higher layer if at all. I'm updating my ideal specification accordingly.

**What would change my mind back**: A use case where the two scores have a *computational dependency* that can't be captured by a shared mapper — i.e., score B requires score A's value as input. I don't have such a case.

## Challenging the Team: Are We Being Circular on Typed Scores?

The team has converged on "typed scores are better" (my Round 1 included). I want to stress-test this because my ideal spec may have been unconsciously shaped by evalkit's existing design.

**The steelman for verda's f64-only approach:**

1. **Low-level libraries should be minimal.** Typed scores push interpretation into the framework. A `Score(f64)` with `Goal::Maximize` is the maximally composable primitive — consumers decide what 0.0/1.0 means.

2. **Real ecosystem evidence**: Braintrust (production scale, thousands of users) uses f64 scores exclusively. OpenAI's eval framework reduces everything to numeric. The tools that *succeed at scale* tend toward numeric simplicity.

3. **Type-appropriate aggregation is a feature, not a requirement.** Wilson CI for binary is great, but you can compute it on f64 scores where values are {0.0, 1.0}. You detect binary-ness at aggregation time rather than encoding it in the type system. Less elegant, more flexible.

4. **The Label score variant is the weakest case.** In my UC-11 (medical diagnostics), what do you actually *do* with `Score::Label("pneumonia")`? You can't compute a mean. You can compute a distribution — but a `HashMap<String, usize>` is sufficient. The Label variant adds type complexity for a use case that's better served by custom per-domain analysis.

**My counter-counter-argument (why I still favor typed scores):**

The killer argument is **direction-aware statistical testing**. evalkit's comparison layer dispatches on score type to choose the right test: Fisher's exact for Binary, Welch's t for Numeric. With f64-only, you either (a) always use t-test (wrong for binary data with small samples), or (b) add a separate "score type" annotation that's effectively the same thing as a typed enum but stringly-typed.

**Updated position**: Typed scores are correct, but the type system should be *minimal*. I'd keep `Numeric(f64)` and `Binary(bool)`. I'd drop `Label(String)` (too niche) and fix `Metric` per Agent 0's suggestion (remove the `name` field). This gives you correct statistical dispatch without the full 4-variant complexity.

**What would change my mind**: If someone shows a common use case where Label-typed statistical analysis (distribution, mode) is needed at the library level rather than in user code.

## Ecosystem Grounding for the Recommendation

The team is converging on "continue evalkit." Let me validate this against ecosystem trajectories:

**Supporting evidence:**
- The tools that survive are the ones with strong statistical foundations (MLflow, Weights & Biases). Statistical rigor compounds over time; it's the hardest thing to bolt on later.
- OTel-native evaluation is an emerging pattern (AgentEvals, EvalForge). evalkit is positioned for this wave; verda is not.
- Spec-driven development with documented deviations is how production libraries maintain stability. Every long-lived eval tool (pytest, JUnit) has a specification or at least a formal plugin contract.

**Counterevidence (what a critic would say):**
- evalkit has 4,200 lines and zero users. verda is leaner. In practice, the library that ships first and iterates wins. evalkit's spec-driven approach may have over-invested in up-front correctness at the cost of shipping speed.
- The OTel observe mode is cool but may be a YAGNI for 90% of users who just want `input → run task → score output`.

**My assessment**: The counterevidence is real but doesn't change the recommendation. The question isn't "which ships faster today" — it's "which foundation supports the broader use case space." evalkit's foundation is stronger. Concurrency (the main gap) is a ~15-line change per Agent 0's evidence.

## Updated Ideal Abstraction (Revisions from Round 1)

Based on Round 2 discussion, three changes to my ideal spec:

1. **Score type**: Keep `Numeric(f64)` and `Binary(bool)`. Drop `Label(String)`. Fix `Metric { value, unit }` (no name — use ScoreDefinition). This is a narrower typed system than Round 1 but still enables correct statistical dispatch.

2. **Scorer return**: Confirmed single `Score` per scorer. ScorerSet handles the shared-computation case.

3. **Acquisition error**: Should be generic `E: Error + Send + Sync`, not a fixed `AcquisitionError` enum. OTel-specific errors behind feature gate. This is Agent 0's point and I endorse it — the leak is worse than I initially assessed.

## Remaining Open Questions

1. **For Agent 2**: Agent 0 claims concurrency in evalkit is a ~15-line change because all types are `Send + Sync`. Can you validate this from verda's implementation experience — were there any concurrency gotchas beyond the obvious `buffer_unordered`?

2. **For Agent 3**: On the builder complexity question — Agent 0 didn't address this. The typestate builder produces good compile-time errors, but does it also produce *terrible* error messages when things go wrong? Rust's typestate errors are notoriously opaque. Is there ecosystem precedent for typestate builders in production Rust libraries?

3. **For the team**: We haven't discussed **versioning and stability**. evalkit uses `#[non_exhaustive]` on some types but not all. verda uses it more consistently. For a library claiming "stable abstraction," every public enum and struct should be `#[non_exhaustive]`. This is a small but important migration item.

---

**CONVERGENCE STATUS**: The team is converging on the right recommendation (continue evalkit) with the right migration items. The single-vs-multi-score debate is now resolved in favor of single (Agents 0, 1, 3's Agent 3 may update). The remaining substantive disagreements are on Score type system minimality (I now propose dropping Label) and migration effort estimates. I don't think further rounds from me would add significant value beyond the open questions above.

I'm close to DELIBERATION COMPLETE but want to see Agent 2's response on the concurrency validation question and Agent 3's updated synthesis before declaring done.
