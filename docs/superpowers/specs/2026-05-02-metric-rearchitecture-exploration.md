# Metric / Measurement Rearchitecture — Design Exploration

**Date:** 2026-05-02
**Status:** Design exploration — not yet a finalized spec
**Successor to:** the metric-handling instinct that produced `Score::Metric` in pre-2.0 work
**Context:** brainstorming + plan-eng-review + two rounds of multi-agent pressure testing

This document captures the full design conversation, the agents' findings, and the locked decisions vs open questions. It is intentionally narrative — the path mattered as much as the destination.

---

## 1. Problem statement

The user articulated the distinction concretely:

> **Scoring is relative** (to some ideal value, often normalized [0,1]).
> **Metrics are absolute values** (counts, durations, costs).
>
> "I'd like to be able to see token count and latency improvements for different prompts or how an agent behaves over time with different prompts (e.g., from traces). My assumption was: metrics."

Use cases driving this:
- Token usage and latency across samples
- Improvements across different prompts
- Agent behavior over time from traces
- Custom metrics emitted from sources (e.g., turn count, response length)

The 2.0 work shipped `ProductionOutput<O>` with typed `usage` / `cost_usd` / `latency` fields, but those are limited to the three well-known measurements. Custom metrics had no first-class home.

---

## 2. Initial investigation: what's already there?

`grep` revealed:

- **`Score::Metric` is unused in production scorer crates.** None of `evalkit-scorers-llm`, `-text`, `-rag`, `-embed`, `-redteam` emit it. Only test fixtures touch it. (Important caveat: this was about *production scorer emission*, not workspace references — the variant is still wired into stats, comparison, exporters, server, runtime — 24 references across 8 files.)
- **The 2.0 envelope (`ProductionOutput`) has typed convenience fields** but no extension point for custom metrics.
- **`SampleResult` aggregates resources via `ResourceUsage`** (typed) at the run level. ~25 dependent sites across the workspace.

This shifted the framing from "how do we restructure metrics" to "do `Score::Metric` and the typed envelope fields belong where they are?"

---

## 3. First proposal — `Measurer` trait

Initial design:

1. New `Measurement { value: f64, unit: Option<String> }` type.
2. Source-side: `ProductionOutput.measurements: HashMap<String, Measurement>` for intrinsic values (turn count, retries, model_id, time-to-first-token).
3. Scorer-side: NEW `Measurer<I, O, R>` trait parallel to `Scorer`, returning `HashMap<String, Measurement>` per trial. For derived values (response_length, edit_distance, BLEU). Attached via `.measurer(MyMeasurer)` on `Run` builder.
4. Both channels merge into `TrialResult.measurements`.
5. `SampleResult.measurement_aggregates` auto-computes mean / stddev / min / max / sum.
6. `Comparison` extends to diff measurements.
7. `Score::Metric` removed.

User pushback: *"I'm not sure measurements should be mixed with OutputSources. An output source could read from otel, but a measurement may need to be calculated. I don't know. I think I can't imagine how the user experience would look like."*

This triggered the first multi-agent pressure test.

---

## 4. Round 1 — four-agent pressure test

Four agents in parallel: industry comparison, adversarial pressure test, DX walkthrough, alternative architectures.

### 4.1 Findings (convergent across all four)

**No peer framework has a separate `Measurer` trait.** Inspect AI, Promptfoo, DeepEval, Braintrust, LangSmith, OpenAI Evals all funnel derived signals through the same scoring API. Adding `Measurer` would make evalkit unique in a way users won't expect.

**"Metric" beats "Measurement" as industry naming.** MLflow, W&B, Promptfoo, DeepEval, Braintrust, LangSmith — all use "metric." "Measurement" is rare in eval-tool vocabulary.

**The intrinsic/derived split is largely fictional.** Agent #2 said it bluntly: "post-hoc justification for two channels that don't need to exist." Agent #3 demonstrated it: `tokens_per_turn` (derived but needs source-emitted data) doesn't even work in the proposal because `ScorerContext` doesn't expose source measurements.

### 4.2 Pressure-test critical findings

- **Naming collision** between `ProductionOutput`'s typed `latency` field and a hypothetical `measurements["latency"]`. No reconciliation rule.
- **`Measurer` re-creates the trait duplication** that `Score::Metric` was originally introduced to eliminate.
- **`Score::Metric` removal** touches 12+ files, third schema break in a month.

### 4.3 Alternative architectures explored

- **Alternative A — `ScoreKind` on `ScoreDefinition`:** unified channel through `Score::Numeric`, semantic split via metadata.
- **Alternative C — Postprocessor pipeline:** generalizes derived enrichment into a separate stage.
- **Alternative E — Source-side wrapping with `.derive()`:** zero new traits, combinator on `OutputSource`.

Agent #4's recommendation: hybrid of A + E. Keep current proposal's data model, drop the `Measurer` trait, add `OutputSourceExt::derive`.

---

## 5. Revised proposal

After the four-agent feedback:

### 5.1 Decisions

- **Drop `Measurer` trait.** `Scorer` does both jobs.
- **Naming: `metric` / `Metric`** instead of `measurement` / `Measurement`.
- **Add `ScoreKind`, `unit`, `range`, `Aggregator` to `ScoreDefinition`.**
- **Source-side: `ProductionOutput.metrics: Vec<SourceMetric>`** for typed source emissions.
- **Add `OutputSourceExt::derive(...)` combinator** for post-output computation.

### 5.2 Types

```rust
pub enum Score {
    Numeric(f64),
    Binary(bool),
    Label(String),
}

pub struct ScoreDefinition {
    pub name: String,
    pub direction: Option<Direction>,
    pub kind: ScoreKind,           // NEW
    pub unit: Option<String>,      // NEW
    pub range: Option<(f64, f64)>, // NEW
    pub aggregator: Aggregator,    // NEW (later debated)
}

pub enum ScoreKind {
    Evaluation,    // gates pass/fail; participates in scoring narrative
    Measurement,   // descriptive observation; aggregated and diffed but not gated
}

pub enum Aggregator {
    Mean,
    Sum,
    Min,
    Max,
    Last,
    Mode,
    None,
}

pub struct SourceMetric {
    pub def: ScoreDefinition,
    pub value: f64,
}
```

### 5.3 `ProductionOutput` collapse

```rust
pub struct ProductionOutput<O> {
    pub output: O,
    pub metrics: Vec<SourceMetric>,
    pub metadata: HashMap<String, Value>,
}
```

`with_usage(TokenUsage{...})` becomes sugar that pushes four reserved-name entries (`tokens.input`, `tokens.output`, `tokens.cache_read`, `tokens.cache_write`). `with_cost_usd(0.01)` pushes `source.cost_usd`. `with_latency(d)` pushes `source.latency_ms`.

### 5.4 Derived combinator

```rust
trait OutputSourceExt<I, O>: OutputSource<I, O> + Sized {
    fn derive<F>(self, def: ScoreDefinition, f: F) -> Derived<Self, F>
    where F: Fn(&I, &O) -> f64 + Send + Sync;

    fn derive_with_reference<R, F>(self, def: ScoreDefinition, f: F) -> DerivedRef<Self, R, F>
    where F: Fn(&I, &O, Option<&R>) -> f64 + Send + Sync;
}
```

User code:

```rust
let source = my_task
    .derive(ScoreDefinition::measurement("response_length").unit("chars"),
            |_, output: &String| output.len() as f64)
    .derive_with_reference(ScoreDefinition::measurement("edit_distance"),
            |_, out, gold: Option<&String>| {
                gold.map(|g| levenshtein(out, g) as f64).unwrap_or(0.0)
            });
```

---

## 6. Plan-eng-review — Step 0 scope challenge

Three findings raised in plan-eng-review of the revised proposal:

1. **Q1: Collapse typed fields fully?** Recommended B (full collapse).
2. **Q2: Is `Score::Metric` already removed?** Claimed yes (incorrectly — it was only `Score::Structured` that was removed in 2.0). No action needed.
3. **Q3: Should `Aggregator` live on definition?** Recommended A (drop, compute all aggregates universally).

The user dispatched a second round of agents to pressure-test these.

---

## 7. Round 2 — three-agent verification

### 7.1 Agent 1: Blast radius for Q1 (typed field collapse)

**Verdict: Hybrid (option 3) — surgical collapse.**

Mapped the actual blast radius:

- `ProductionOutput.usage`: **1 non-test production reader** (`run.rs:199-216` funnel).
- `ProductionOutput.cost_usd`: **1 non-test production reader** (same funnel).
- `ProductionOutput.latency`: **1 non-test production reader** (same funnel).
- `SampleResult.source_resources` / `scorer_resources`: ~25 sites across 13 files, all using `ResourceUsage::default()` builders. No assumption about typed `ProductionOutput` shape.
- Plugin protocol: zero references to typed field names.
- Schema: describes `ResourceUsage`, not `ProductionOutput`.
- Migration tools: defaulting `source_resources`/`scorer_resources`. No v3→v4 work needed.
- Test fixtures: hardcoded shape — zero.

**Recommendation:** Replace `ProductionOutput`'s three typed fields with `metrics: Vec<SourceMetric>` (~5 file edits). **Keep `ResourceUsage` typed** as the canonical aggregation home in `SampleResult`. The funnel at `run.rs:199` translates reserved metric names back into typed `ResourceUsage` fields.

The two layers serve different jobs. Producer boundary wants extensibility (custom metrics). Aggregation boundary wants typed sums (cost_usd as `f64`, not parsed-from-string).

### 7.2 Agent 2: Aggregation strategy for Q3

**Verdict: Hybrid (option 3) — producer-declared default with consumer-override escape hatch.**

Peer practice splits cleanly by domain:

| Framework | Producer-declared aggregator? | Notes |
|---|---|---|
| MLflow | No | Generic experiment tracking; consumer picks at query time |
| W&B | No (with optional summary hint) | Same |
| Prometheus | **Yes (mandatory)** | Counter / gauge / histogram drives correctness in PromQL |
| OpenTelemetry | **Yes (mandatory)** | Counter / Gauge / Histogram drives temporality + aggregation |
| Inspect AI | **Yes** | Reductions (`mean`, `accuracy`, `stderr`) declared by scorer author |
| Braintrust | No | Scores at log time, dashboards pick at query time |
| Datadog | Yes (type), no (function) | Type producer-declared, function consumer-picked within constraints |

Why production observability requires producer-declared types: correctness depends on it (`rate()` only valid on counters; histogram bucketing math).

Why generic experiment tracking doesn't: no distributed aggregation across replicas, no monotonic counter semantics, no `rate()`.

**Inspect AI — evalkit's closest peer — goes producer-declared.** Eval metrics carry semantic intent: `accuracy` is mean-of-booleans, `stderr` needs sample variance, `at_least(k)` is a thresholded count. Forcing every metric to surface `min/max/sum` produces meaningless numbers in reports.

The "categorical-encoded `model_id`" argument I had used to justify "compute all" was **weak**. No peer encodes strings as f64. Hashing a string into f64 is an evalkit smell, not a justification.

**Recommendation:** Keep `Aggregator` on `ScoreDefinition` as the **declared default** (mandatory). Always also store **raw samples** so consumers *can* override at read time. Reject the "compute all five universally" version. Matches Datadog's model: **type producer-declared, function consumer-picked within constraints.**

### 7.3 Agent 3: `Score::Metric` removal verification

**Verdict: Claim was FALSE.** I had confused `Score::Metric` removal with `Score::Structured` removal.

- `grep -rn "Score::Metric"` in Rust source: **24 references across 8 files** (kernel, exporters, server, runtime, tests).
- `Score` enum has **four** variants (Numeric, Binary, Label, Metric).
- Schema doc still lists `metric` as a Score variant.
- Migration tool handles `Score::Structured` → `Score::Numeric` but **not** `Score::Metric`.

This corrected my Step 0 finding. Real decision required on whether to remove or keep.

---

## 8. Final synthesis

### 8.1 Locked (high confidence after both rounds of agent review)

**Q1 — Surgical collapse, not full collapse.**

- Replace `ProductionOutput`'s three typed `Option` fields with `metrics: Vec<SourceMetric>` at the producer boundary.
- Funnel at `run.rs:199-216` translates reserved metric names back into typed `ResourceUsage`.
- Keep `ResourceUsage` typed as canonical aggregation home.
- ~5 file edits, no schema change, no migration tool change, no plugin-protocol change.

**Q3 — Producer-declared aggregator with consumer-override.**

- Keep `Aggregator` on `ScoreDefinition`. Mandatory at metric definition (no default).
- Store raw `samples: Vec<f64>` per metric so consumers can override at read time.
- Reject "compute all five universally."
- Inspect AI / Datadog pattern: producer declares intent; consumer can pick a different lens if they have reason.

**Other locked decisions:**

- `ScoreKind`, `unit`, `range`, `Aggregator` added to `ScoreDefinition`.
- `OutputSourceExt::derive` combinator for derived measurements.
- No `Measurer` trait (`Scorer` does both jobs via `ScoreKind`).
- Naming: `Metric` / `metric` (industry alignment).
- Sugar methods on `ProductionOutput` (`with_usage`, `with_cost_usd`, `with_latency`) stay as compatibility shims that push reserved-name entries.

### 8.2 Open question

**Q2 — Remove `Score::Metric` as part of this work, or keep?**

Arguments for remove:
- With `ScoreDefinition.unit` and `Aggregator` carrying intent, `Score::Metric { name, value, unit }` becomes redundant with `Score::Numeric(value)` + definition-side metadata.
- The variant currently does double duty — `name` duplicates `ScoreDefinition.name`.
- 24 references is mechanical (most are match arms collapsing to `Numeric`).

Arguments for keep:
- 24 references is real work even if mechanical.
- Working today; not actively harmful.

The user said breaks don't matter. Lean toward remove, but explicit decision still needed before locking the design.

### 8.3 Other open / deferred items

- Memory concern: raw `samples: Vec<f64>` per metric for large runs. Solvable with reservoir sampling beyond N if it bites in practice. Don't design around it now.
- Architecture-review issues from plan-eng-review (`f64`-only value, origin tracking, derive composition with Mapper, `SourceMetric` definition duplication, Arc-ing) — pending the rest of the eng review.
- Code-quality, test, and performance review sections of plan-eng-review — pending Step 0 sign-off.
- `MetricSet` analogous to `ScorerSet` — open question, probably "no" if `derive` chains are sufficient.

---

## 9. What this design buys

**For the user's stated use cases:**

| Use case | How it's served |
|---|---|
| Track token usage / latency across samples | Source emits via `with_usage` / `with_latency` sugar → reserved metric entries → `Comparison` diffs across runs |
| Compare prompts on token / latency | Same, via two `Run`s on the same dataset → `Comparison.shared_metrics` |
| Agent behavior from traces | `OtelObserver` extracts metrics from spans → emits via `ProductionOutput.metrics` → uniform aggregation |
| Custom metric: turn count | Source closure pushes `SourceMetric { def: …, value: turns as f64 }` |
| Custom metric: response length | `my_task.derive(ScoreDefinition::measurement("response_length"), \|_, o\| o.len() as f64)` |
| Custom metric: edit distance | `.derive_with_reference(ScoreDefinition::measurement("edit_distance"), \|_, o, r\| ...)` |

**For the kernel:**

- One channel at the producer boundary; typed aggregation at the result boundary.
- `Scorer` trait surface unchanged. No new traits.
- `Comparison` becomes uniform across evaluation scores and measurement metrics, with `direction` driving improvement labels.
- `Score::Metric` removal (if approved) eliminates the double-duty `name` redundancy.

**Net architectural cost:**

- New types: `ScoreKind`, `Aggregator`, `SourceMetric`, `MetricAggregate`, `MetricComparison`, `Derived` / `DerivedRef`.
- Modified: `ScoreDefinition` (4 new fields), `ProductionOutput` (typed fields → `metrics`), `TrialResult` (gains `metrics`), `SampleResult` (gains `metric_aggregates`), `Comparison` (gains `shared_metrics`).
- Removed: `Score::Metric` (pending Q2 sign-off).
- Schema breaks: bump v3 → v4. Migration tool extends with field-fold + Score::Metric → Numeric transform.
- New traits: zero.

---

## 10. Path to spec

Once Q2 lands:

1. Write the formal spec at `docs/superpowers/specs/2026-05-XX-metric-rearchitecture-design.md`.
2. Run remaining plan-eng-review sections (Architecture, Code Quality, Tests, Performance).
3. Lock the spec, write the implementation plan via `superpowers:writing-plans`.
4. Execute via `superpowers:subagent-driven-development`.

---

## Appendix A — alternatives considered and rejected

- **Full collapse of `ResourceUsage` into metrics-everywhere.** Rejected: 25+ aggregation sites depend on typed `ResourceUsage`; schema commits to it; the win is at the producer boundary, not the aggregation boundary.
- **Drop `Aggregator` entirely, compute mean/sum/min/max/last universally.** Rejected: produces nonsense for non-sum metrics, pollutes report columns, contradicts Inspect AI / OTel / Prometheus precedent for typed-intent.
- **`Measurer` trait parallel to `Scorer`.** Rejected: re-creates trait duplication that `Score::Metric` was meant to eliminate; no peer framework does this; `Scorer` + `ScoreKind` covers the same use cases.
- **"Measurement" naming.** Rejected: industry uses "metric" universally; `Measurement` is rare in eval-tool vocabulary.
- **Postprocessor pipeline.** Rejected: re-introduces a near-duplicate trait under a different name; adds a third decision point ("scorer vs measurer vs postprocessor"); not justified by current use cases.
- **OTel meter integration as the metrics channel.** Rejected: federation to OTel for first-class data is the wrong layering; evalkit is generic, OTel is one consumer.

## Appendix B — terminology

| Term | Meaning |
|---|---|
| Score | An evaluation outcome (Numeric / Binary / Label) |
| Metric | A descriptive measurement (always numeric in this design) |
| Evaluation (`ScoreKind::Evaluation`) | A score that gates pass/fail; participates in the scoring narrative |
| Measurement (`ScoreKind::Measurement`) | An observation that's aggregated and diffed but not gated |
| Source-emitted metric | Pushed via `ProductionOutput.metrics` by a `Task` / `OtelObserver` / HTTP / subprocess source |
| Derived metric | Computed via `OutputSourceExt::derive(...)` post-output |
| Scorer-emitted score | Returned from `Scorer::score()` as before |
| Reserved metric name | A well-known name (`tokens.input`, `source.cost_usd`, etc.) populated by sugar methods on `ProductionOutput` |
