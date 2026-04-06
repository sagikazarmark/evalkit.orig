---
agent: agent-1
role: ""
phase: phase2
round: 1
seq: 010
timestamp: 2026-04-06T06:15:05Z
---

Now I have enough context. Let me synthesize my deliverables.

---

# Agent 1 — Eval Researcher: Independent Analysis (Round 1)

## Deliverable 1: Use Case Catalog

I've designed 12 use cases spanning AI eval, traditional software testing, and non-AI domains. Each is grounded in real-world patterns from the 65+ tools surveyed and my fresh research.

### AI Evaluation Use Cases

| # | Use Case | Description | Key Abstraction Demands |
|---|----------|-------------|------------------------|
| **UC-1** | **LLM output correctness** | Compare model answers against gold references (exact, contains, regex). The "hello world" of evals. | Sample(input, reference), Scorer → Binary/Numeric, Dataset, single trial |
| **UC-2** | **LLM-as-a-Judge** | Use a second model to grade output quality (relevance, faithfulness, coherence). Dominant pattern across DeepEval, Braintrust, Inspect AI. | Scorer that calls external API, async, Score as numeric or label, error handling for judge failures |
| **UC-3** | **Non-deterministic agent multi-trial** | Run an agent 10× on same input, compute pass_at_k, confidence intervals, significance tests. The critical gap identified in prior research (only Agentrial addresses this). | Multi-trial execution, statistical aggregation by score type, comparison with significance testing |
| **UC-4** | **Agent trajectory evaluation via OTel traces** | Agent already ran in production; evaluate by querying its OTel traces (tool calls, reasoning steps) rather than re-executing. Pattern from AgentEvals (aevals.ai). | Observe-mode acquisition (trace-based), span extraction, scorer receives structured traces not raw text |
| **UC-5** | **Brain dump agent classification improvement** | User's stated case: an agent that segments/classifies thoughts. Iteratively improve classification quality by running eval → tweaking prompt → comparing runs. | Run-to-run comparison, per-sample regression detection, iteration workflow |
| **UC-6** | **Multi-scorer evaluation with shared transforms** | RAG pipeline: score faithfulness, relevance, and groundedness on same output. Output needs JSON parsing before scoring. | ScorerSet with shared mapper/transform, multiple scores per sample, per-scorer aggregation |

### Traditional Software Testing Use Cases

| # | Use Case | Description | Key Abstraction Demands |
|---|----------|-------------|------------------------|
| **UC-7** | **API endpoint regression testing** | Run a test suite against HTTP endpoints, score response correctness, latency, schema conformance. Compare releases. | Acquisition = HTTP call, Metric (latency with units), comparison across runs, parallelism |
| **UC-8** | **Compiler/parser fuzz testing with scoring** | Feed generated inputs to a parser, score by correctness + performance. Track pass rates over time. | Large datasets, Binary scores, performance Metrics, statistical aggregation across thousands of samples |
| **UC-9** | **Data pipeline quality monitoring** | Score ETL output quality: completeness, freshness, schema conformance. Non-AI but needs identical abstractions. | Numeric scores, custom acquisition (query data source), comparison for drift detection |

### Non-AI Domain Use Cases

| # | Use Case | Description | Key Abstraction Demands |
|---|----------|-------------|------------------------|
| **UC-10** | **A/B test analysis** | Two variants of a product feature, measure conversion metrics across user samples. | Two runs (baseline/candidate), statistical significance testing, per-sample comparison |
| **UC-11** | **Medical diagnostic accuracy** | Score a diagnostic system's predictions against ground truth labels. Requires Label scores (categories), not just numeric. | Label scores with distribution analysis, multi-class confusion tracking, per-sample detail |
| **UC-12** | **Content moderation quality** | Score a moderation system: precision/recall on labeled test set, with Binary pass/fail per sample and Numeric confidence. | Mixed score types (Binary + Numeric) from same run, aggregation must handle both |

### Critical Observations About Use Cases

1. **UC-3 and UC-4 are the differentiating cases** — these are where the gap in the ecosystem is widest and where the library's value proposition lives.
2. **UC-7 through UC-10 are the "generic" litmus test** — if the library can't serve these without AI terminology leaking in, the "domain-agnostic" claim fails.
3. **UC-11 and UC-12 stress the score type system** — they require Label and mixed score types, which is where evalkit and verda differ most fundamentally.

---

## Deliverable 2: Updated Landscape Survey

Building on the existing 65+ tool survey (which is thorough and I largely endorse), here are **fresh signals from April 2026**:

### New Entrants & Shifts Since Prior Research
- **EvalForge** — Framework-agnostic eval harness that accepts trace JSON, scores via CI pipeline. Validates the trace-based eval pattern that evalkit's observe mode targets.
- **AgentEvals (aevals.ai)** — Now a proper product, not just a GitHub project. Scores agent behavior directly from OTLP streams and Jaeger JSON. Validates the OTel-native approach.
- **MLflow GenAI evaluate()** — Databricks now has built-in `mlflow.genai.evaluate()` with typed datasets, scorers, and expectations. Validates the Dataset → Task → Scorer pipeline pattern as an industry consensus.
- **Braintrust multi-SDK expansion** — Go, Ruby, C#, Java SDKs. Shows that the abstraction pattern (data + task + scorer) translates across languages.
- **Inspect AI maturity** — Now at production scale with Docker sandboxing, VS Code integration. The Dataset → Solver → Scorer pattern is the most mature open-source reference architecture.

### Ecosystem Consensus on Core Abstractions

Across all surveyed tools, the **convergent pattern** is:

```
Dataset/TestCases → Task/Solver/Acquisition → Scorer/Grader/Evaluator → Score/Result
```

Every mature tool has these four concepts, though naming varies wildly. The key architectural disputes are:
1. **Score type**: Numeric-only (Braintrust, verda) vs. typed enum (evalkit's Numeric/Binary/Label/Metric) vs. string-tagged (Inspect AI)
2. **Acquisition coupling**: Framework calls the task (Inspect AI Solver, Braintrust task) vs. framework queries traces (AgentEvals, evalkit observe mode) vs. both (evalkit)
3. **Stats integration**: Embedded in results (verda, Braintrust) vs. separate computation (evalkit)
4. **Comparison**: First-class with significance testing (evalkit) vs. simple delta (verda, most others) vs. platform-side (Braintrust, LangSmith)

---

## Deliverable 3: Ideal Abstraction Specification

Based on all 12 use cases and the ecosystem consensus, here is what an ideal low-level eval library must provide:

### Core Types

```
Sample<I, R>           — Input + optional reference + stable ID + tags + metadata
Dataset<I, R>          — Ordered collection of Samples; trait-based for custom materialization
Score                  — MUST be typed (not just f64); at minimum: Numeric(f64), Binary(bool), Label(String)
                         Rationale: UC-11, UC-12 demand non-numeric scores; typed scores enable
                         type-appropriate aggregation (pass_rate vs mean vs distribution)
ScoreDefinition        — Name + Direction(Maximize|Minimize); first-class, not ad-hoc
Scorer<I, O, R>        — async fn score(context) → Result<Score, ScorerError>
                         Returns single score per scorer (1:1 mapping enables clean definition())
                         Multiple scores = multiple scorers (composable, not overloaded)
ScorerContext<I, O, R> — Borrowed references to input, output, reference; non_exhaustive for extension
Acquisition<I, O>      — async fn acquire(input) → Result<O, AcquisitionError>
                         Blanket impl for closures; custom impls for OTel trace-based acquisition
Mapper<I, O>           — Synchronous transform for output/reference adaptation before scoring
```

### Execution

```
Run::builder()
  .dataset(...)
  .acquisition(...)
  .scorer(...)           — Additive; at least one required
  .scorer_set(...)       — Optional: grouped scorers with shared mappers
  .trials(N)             — Default 1
  .build()? → Run
  .execute().await? → RunResult
```

### Results & Statistics

```
RunResult              — Raw per-sample, per-trial, per-scorer results (no aggregation embedded)
RunStats               — Computed from RunResult; type-appropriate aggregation:
                         Numeric → mean, stddev, CI, min, max
                         Binary → pass_rate, pass_at_k, CI (Wilson)
                         Label → distribution, mode
Comparison             — Baseline vs candidate; per-scorer, per-sample deltas;
                         significance testing (t-test for Numeric, Fisher's for Binary)
                         Direction-aware interpretation
```

### Key Design Principles (from ecosystem patterns)

1. **Typed scores are essential, not optional.** Every tool that starts with f64-only eventually needs labels and booleans. Doing it up front avoids breaking changes. The ecosystem evidence: Inspect AI uses strings, Braintrust uses floats and regrets it for categorical evals, DeepEval has `MetricScore` with pass/fail.

2. **1:1 Scorer-to-Score mapping.** A scorer should return one `Score`, not `Vec<Score>`. This enables clean `definition()` metadata and prevents runtime name-collision errors. If you need multiple scores, compose multiple scorers.

3. **Stats must be decoupled from results.** RunResult is raw data; stats are derived. This is the MLOps consensus — raw data is the source of truth, aggregation is a view.

4. **Acquisition as first-class trait, not just closures.** The OTel observe-mode use case (UC-4) demands a structured acquisition that can query trace backends. A blanket impl for closures handles the common case.

5. **Comparison with statistical significance.** Simple deltas are insufficient for non-deterministic systems (UC-3, UC-10). This is the critical gap in the ecosystem.

6. **Error ≠ low score.** Infrastructure failures (scorer timeout, API error) must be distinguishable from poor performance (low score). Both libraries get this right.

7. **Tracing support must be first-class.** Either via OTel integration or via structured trace ingestion. The library doesn't need to *be* an observability tool, but it must *interoperate* with one.

---

## Critical Assessment: Where the Libraries Stand Against the Ideal

### evalkit vs. Ideal Specification

| Dimension | Alignment | Notes |
|-----------|-----------|-------|
| Typed Score enum | ✅ Strong | Numeric/Binary/Label/Metric — matches ideal exactly |
| 1:1 Scorer:Score | ✅ Strong | `score() → Result<Score, ScorerError>` — clean |
| ScoreDefinition | ✅ Strong | First-class with Direction |
| Acquisition trait | ✅ Strong | With blanket impl + OTel observe mode |
| Mapper | ✅ Strong | Unified transform for output + reference |
| Stats decoupled | ✅ Strong | RunResult raw, RunStats computed |
| Statistical comparison | ✅ Strong | t-test, Fisher's exact, CI, Change enum |
| ScorerSet | ✅ Good | Shared mappers reduce boilerplate |
| OTel observe mode | ✅ Unique | Only library with trace-based acquisition |
| Parallelism | ⚠️ Deferred | Sequential only (designed but not implemented) |
| Tracing | ⚠️ Unclear | OTel for acquisition, but what about run-level tracing? |

### verda vs. Ideal Specification

| Dimension | Alignment | Notes |
|-----------|-----------|-------|
| Score type | ❌ Weak | f64-only. Cannot represent Binary pass/fail or Label categories natively. Forces UC-11, UC-12 into numeric encoding. |
| 1:1 Scorer:Score | ❌ Weak | `score() → Vec<Score>` — multi-score return creates runtime name-collision risk, prevents clean definition(), complicates aggregation |
| Score direction | ✅ Good | Goal enum on Score itself |
| Acquisition | ❌ Missing concept | Task function is a raw closure `Fn(I) → Fut<Result<R, E>>`. No Acquisition trait, no observe mode possible. |
| Mapper | ❌ Missing | No transform layer between acquisition and scoring |
| Stats decoupled | ⚠️ Partial | Stats embedded in RunResult (ScoreSummary, MetricSummary) — tightly coupled |
| Statistical comparison | ⚠️ Weak | Simple deltas, no significance testing, no CI |
| Parallelism | ✅ Strong | First-class with configurable parallelism |
| Metrics (latency, tokens) | ✅ Good | Separate Metric type with units — nice for UC-7, UC-8 |
| Metadata/Tags | ✅ Good | Rich per-sample metadata, tags for grouping |
| Persistence | ✅ Good | JSON roundtrip for all result types, typed decode |
| Error handling | ✅ Strong | Comprehensive error enum with validation |

### The Critical Divergence: Score Type System

This is **the single most consequential design difference** between the two libraries, and I want to be very clear about why it matters:

**verda's f64-only scores** work for the common case (UC-1, UC-2, UC-3, UC-5, UC-7). But they fail for:
- **UC-11 (medical diagnostics)**: You can't meaningfully represent "diagnosis: pneumonia" as a float. You need Label scores with distribution analysis.
- **UC-12 (content moderation)**: Mixed Binary (blocked/allowed) + Numeric (confidence) from the same run requires typed dispatch for aggregation.
- **UC-4 (trajectory eval)**: Agent step classifications ("correct_tool", "wrong_tool", "hallucinated") are naturally Labels, not floats.

The ecosystem evidence supports typed scores: Inspect AI uses typed `Value` in scores, Braintrust reports wanting categorical support, DeepEval has `MetricScore` with both numeric and pass/fail semantics.

**However, a well-informed critic would argue:** f64-only is simpler, covers 80% of cases, and users can encode booleans as 0/1 and labels as separate numeric scorers. The complexity cost of a typed Score enum is real (every consumer must handle all variants). *What would change my mind:* if I saw evidence that real users of eval libraries consistently work around typed scores and prefer numeric-only. I haven't seen that evidence — instead I see tools adding types over time.

### The Second Critical Divergence: Acquisition Abstraction

evalkit's `Acquisition<I, O>` trait with OTel observe mode is a genuine architectural innovation that no other Rust library provides. verda's raw closure approach is simpler but cannot support UC-4 (trace-based evaluation) without a breaking redesign.

The ecosystem validates this: AgentEvals (aevals.ai) is now a product built entirely around trace-based scoring. EvalForge accepts trace JSON. This is a growing pattern, not a niche.

**However:** verda's simpler closure approach is more ergonomic for the 90% case (UC-1 through UC-3). The Acquisition trait with its `AcquisitionError` enum adds ceremony. *What would change my mind:* if observe-mode turns out to be a rarely-used feature that adds permanent API complexity for little gain.

---

## What Evidence Would Change My Assessment

1. **If real users consistently prefer f64-only scores** and work around the lack of types without friction → verda's simpler Score wins.
2. **If OTel observe-mode is used by <5% of users** → the Acquisition trait overhead may not be justified.
3. **If verda's parallelism proves critical for production use** → evalkit's sequential-only execution is a serious gap, not just a deferral.
4. **If the ScorerSet abstraction proves confusing in practice** → the Mapper layer may be over-engineering.

---

## Preliminary Positioning (subject to revision after seeing other agents' analysis)

**evalkit has the stronger foundational abstractions.** Its typed Score enum, Acquisition trait, Mapper layer, decoupled stats, and statistical comparison align more closely with the ideal specification and ecosystem trajectory. It was designed with the harder use cases (UC-3, UC-4, UC-11, UC-12) in mind.

**verda has real strengths to preserve:**
- Parallelism (evalkit needs this)
- Metrics as a separate concept from Scores (latency, token count — evalkit folds these into the Score::Metric variant, which is arguably less clean)
- Metadata/tags on samples (richer than evalkit's)
- JSON persistence with typed decode
- Display impl on Comparison (nice DX)
- `#[non_exhaustive]` on all public types (good API stability practice)
- `async_trait` usage (more compatible with current Rust ecosystem vs evalkit's `#[allow(async_fn_in_trait)]`)

I defer to Agent 0 on evalkit's spec conformance details, to Agent 2 on verda's implementation quality, and to Agent 3 on the final recommendation. My contribution is the use cases, ideal spec, and ecosystem grounding above.
