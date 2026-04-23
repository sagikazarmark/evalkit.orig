> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Phase 5 — Discoveries

---

## Connections the User Didn't Make

### 1. Self-Instrumenting Eval Framework = Eval Dogfooding

The user suggested using OTel to track eval framework costs. This goes further than cost tracking:

If the eval framework emits OTel spans for its own operations (scorer execution, LLM judge calls, trace collection, etc.), then:
- **Cost tracking** falls out naturally (token counts, API costs as span attributes)
- **Performance profiling** of the eval pipeline itself is free (which scorers are slow?)
- **The framework can evaluate itself** — meta-evaluation becomes trivially possible. You could run scorers on the framework's own traces to detect scorer quality degradation.
- **The OTel observation mode can consume the framework's own traces** — enabling recursive evaluation pipelines

This creates a virtuous cycle: the eval framework is both a producer and consumer of OTel data. The same architecture that evaluates agents evaluates the evaluator.

**Evidence:** Stream 7 identifies "evaluation of evaluators (meta-evaluation)" as an emerging concern. Stream 5 (#3) documents eval cost as a real pain point. No existing tool self-instruments with OTel.

### 2. The Scorer Trait Naturally Supports Pairwise Comparison

The user's scorer model is: `(input, output, reference?) → Score`

Pairwise comparison is: `(input, output_a, output_b) → Score` (which is better?)

These are the same shape if you model it as: `(input, output_a, reference=output_b) → Score`. The "reference" IS the other output. A pairwise scorer is just a scorer where the reference happens to be another output rather than a ground truth.

This means pairwise comparison doesn't require a new abstraction — it's a use pattern of the existing scorer trait. The framework doesn't need to "support" pairwise comparison as a feature; it falls out of the generic design.

**Evidence:** Stream 4 (Decision 2) documents reference-based vs. reference-free evaluation. The user's generic model accidentally supports both plus pairwise.

### 3. The Transform Step Solves the "Artifact Evaluation" Problem

The user's Excalidraw use case (JSON → PNG → visual evaluation) reveals a pattern that applies more broadly:

- Code generation agents: source code → compile → run tests → pass/fail
- Document agents: markdown → render → visual quality check
- Data pipeline agents: config → execute pipeline → validate output data
- API agents: API spec → generate client → compile + test

All of these follow: `raw_output → transform(raw_output) → artifact → score(artifact)`. The transform is domain-specific, but the pattern is universal.

In the user's framework, this maps to: the transform is a function `O → O'` that sits between output acquisition and scoring. The scorer then operates on `O'` (the artifact) rather than `O` (the raw output).

In Rust, this could be as simple as a closure: `transform: Option<Box<dyn Fn(O) -> O'>>` on the evaluation config. Or, more elegantly, transforms ARE scorers — a transform that renders JSON to PNG and then scores the PNG is just a composite scorer.

**Evidence:** Stream 3 (Job 7) documents agent evaluation complexity. The user's specific use cases (Excalidraw, GitHub issue agent) are instances of a broader pattern not addressed by any existing tool.

### 4. The "Observe" Mode Enables Evaluation Without Access to the Agent

This is more powerful than the user may have realized. If the framework can evaluate from OTel traces alone, it enables:

- **Third-party agent evaluation**: Evaluate agents you don't own or control (vendor evaluation, compliance checking)
- **Production evaluation without deployment access**: Quality team evaluates production agents by reading traces, without needing deploy access
- **Historical evaluation**: Re-evaluate past agent runs against new criteria by replaying traces from a trace store
- **Cross-team evaluation**: Platform team provides evaluation infrastructure, product teams run agents — traces flow to eval automatically

This positions the framework differently from every existing tool. Most eval tools require you to own the agent and wrap it. Observe mode only requires access to the traces.

**Evidence:** Stream 3 (Segment 6, Platform/Infrastructure Engineer) describes teams that manage eval infrastructure for others. Stream 7 (Trend 3) shows OTel becoming universal. No existing tool enables "evaluate agents you don't own."

### 5. Serializable Results Enable a "Git for Eval Results" Pattern

The user deferred storage but wants serializable output. If results are JSON files, they can be:
- **Committed to git** alongside the code they evaluate — versioned eval results
- **Diffed** between branches — `git diff main..feature -- eval-results/` shows quality changes
- **Reviewed in PRs** — eval results as PR artifacts, reviewable like code
- **Compared with standard tools** — `jq`, `diff`, custom scripts

This is "evaluation-as-code" taken literally: the eval definition AND the eval results live in the repo. No platform needed. CI/CD comparison becomes `diff last-main-result.json current-result.json`.

**Evidence:** Stream 7 (Prediction 6) forecasts "evaluation-as-code" becoming standard. Stream 4 (Pattern 6) documents declarative/config-driven evaluation. No existing tool uses git as the storage/comparison backend for results.

---

## Layered Opportunity Analysis

### Existing Solutions: Too High-Level or Too Low-Level?

**Too high-level (opinionated products where a library should exist):**
- Braintrust, LangSmith, Maxim: Full platforms. You can't use their scoring logic without their platform.
- DeepEval: Library, but opinionated about pytest integration and AI-specific types. Can't use the scoring primitives without the AI framework.
- Promptfoo: CLI tool with YAML config. Can't use scoring logic programmatically in a different context.

**Too low-level (raw primitives where an ergonomic layer is missing):**
- OTel libraries: Give you raw spans but no evaluation semantics on top
- Generic test frameworks (pytest, cargo test): Give you assertion primitives but no evaluation concepts (scoring, datasets, comparison)

**The gap:** There's no layer between "raw assertion framework" and "opinionated AI eval platform." The user's vision sits exactly in this gap: a typed evaluation library that's more structured than assertions but less opinionated than an AI eval framework.

### Component Decomposition

The user's ideas decompose into layers:

```
Layer 4: AI-Specific Scorers & Harness
  - LLM-as-judge scorer
  - Trajectory scorer
  - Agent harness (run LLM, collect output)
  - AI-specific convenience types (Message, ToolCall, etc.)

Layer 3: Execution Modes & Integration
  - CLI binary (subprocess mode)
  - OTel observation mode (trace collection, extraction)
  - Result comparison & CI/CD gating
  - Statistical multi-trial runner

Layer 2: Core Framework
  - Sample, Dataset, Scorer trait, Score, Result, Run
  - Inline execution mode (call function, score output)
  - Serialization (JSON/JSONL)
  - Transform pipeline (optional)

Layer 1: Primitives
  - Score types (Continuous, Binary, Label, Metric)
  - Aggregation functions (mean, pass rate, confidence intervals)
  - Comparison functions (diff two scores, trend analysis)
```

**Layer 1** is the smallest standalone component — just types and functions. Useful as a dependency for anyone building eval tooling in Rust.

**Layer 2** is the smallest useful framework — you can write and run evaluations.

**Layer 3** adds the execution modes and integration that make it a complete tool.

**Layer 4** adds AI-specific features that make it competitive with DeepEval/RAGAS for AI use cases.

### Shared Foundations

Ideas that share foundational components:

- **I-01 (generic library) + I-02 (multi-mode) + I-03 (OTel)**: All depend on the Layer 2 core (Sample, Scorer, Score types). Building Layer 2 right enables all three.
- **I-04 (flexible scoring) + I-05 (comparison)**: Both depend on the Score type system in Layer 1. Getting Score types right (enum with Continuous/Binary/Label/Metric) enables both scoring flexibility and meaningful comparison.
- **I-06 (transforms) + I-03 (OTel observation)**: Both involve "getting the output" before scoring. The acquisition layer abstraction serves both.

**Highest-leverage build target:** Layer 2 (core framework). It has standalone value AND unlocks everything else.

---

## Conflicts and Tensions

### 1. Generic Core vs. AI Ergonomics

The user wants an AI-agnostic core API. But AI evaluation is the primary use case, and AI users expect AI-native types. The tension:
- Generic: `Scorer<I, O, R>` where I/O/R are arbitrary types
- AI-ergonomic: `scorer(input: &str, output: &str, reference: &str) -> Score`

**Resolution:** Layer 2 is generic. Layer 4 provides AI-specific type aliases and convenience functions. Users who want generic use Layer 2 directly. Users who want AI convenience use Layer 4 imports.

### 2. Python Delay vs. Adoption Risk

The user wants to delay Python SDK. The research says Python is non-negotiable for most users. The tension:
- User priority: build for himself, Rust-first
- Adoption reality: Python ecosystem dominance

**Resolution:** Acceptable trade-off for now. The user is the first customer. Python bindings can be added later when adoption matters. The Restate pattern (PyO3) is proven and can be applied when needed. The key constraint: the Rust API design should not make PyO3 wrapping difficult later (avoid Rust-specific patterns that don't translate, like complex lifetimes in public API).

### 3. OTel as Optional vs. OTel as Core Architecture

The user said OTel shouldn't be a hard requirement (T-04). But the self-instrumentation idea and observe mode both depend heavily on OTel. The tension:
- Core library should work without OTel dependency
- Some of the most compelling features (observe mode, self-instrumentation, cost tracking) require OTel

**Resolution:** Feature-gated OTel. The core library (`evallib`) has zero OTel dependency. An `evallib-otel` crate (or Cargo feature) adds observe mode, self-instrumentation, and trace-based features. Users who don't want OTel get a clean, dependency-light library. Users who want the full power enable the feature.

### 4. Simplicity vs. Flexibility

The user's anti-goals include "so complex that I dread writing new evals." But the full vision (generic types, multiple modes, transforms, multi-score) is inherently complex. The tension:
- Anti-goal: simplicity
- Requirement: flexibility for diverse use cases

**Resolution:** The "good defaults" principle the user described. The simple case should be trivially simple:
```rust
let score = exact_match(output, reference);
```
The complex case should be possible but not forced:
```rust
let run = Run::builder()
    .dataset(samples)
    .scorer(my_custom_scorer)
    .transform(render_to_png)
    .trials(10)
    .build()
    .execute()?;
```
Builder pattern, optional fields, sensible defaults.

---

## Knowledge Gaps

### Researchable (more desk research)
1. What does the Rust OTel library (`opentelemetry-rust`) look like for trace context propagation? Can the framework set `traceparent` headers easily?
2. What JSON schema patterns exist for eval results? Is there any prior art to align with?
3. How does `evalframe` (the Lua+Rust eval crate) design its scorer API? Any patterns to learn from?

### Requires Prototyping
1. **Does the generic Scorer trait work in practice?** Write a few scorers (exact match, regex, LLM-as-judge) and see if the generic API is ergonomic or painful.
2. **Does PyO3-friendliness constrain the Rust API?** Try wrapping the core types with PyO3 to check for lifetime/ownership issues before committing to a public API.
3. **Does trace context propagation actually work end-to-end?** Set up a simple OTel-instrumented HTTP service, send a request with `traceparent`, verify the trace ID propagates through internal spans.
4. **How does JSONL result output feel for comparison?** Generate some result files and try diffing them. Is the format human-readable enough?

### Requires User Interviews (or personal experience)
1. How painful is the transform step in practice? Is "render Excalidraw JSON to PNG" something the user does often, or is it a rare edge case?
2. When working across diverse projects, how much scorer code is reusable? Does the user write new scorers for each project or reuse common ones?

---

## User Segment Reality Check

### Segment 1: AI/LLM Application Engineer (from Stream 3)

"You're a Python developer building a chatbot. You currently use DeepEval. Would you switch to this?"

**Likely answer:** Not initially. DeepEval has 400k monthly downloads, 14.4k stars, built-in metrics, pytest integration. Switching to a Rust tool with no Python SDK is a non-starter. Even with Python bindings, the metric library gap (no pre-built LLM-as-judge, no faithfulness metric, no RAG metrics) makes it hard.

**What would make them switch:** Python SDK + pre-built AI scorers + better statistical rigor than DeepEval. The "multi-trial with confidence intervals" feature could be the wedge.

**Switching cost:** High (learn new tool, rewrite eval suites, lose DeepEval ecosystem).

### Segment 6: Platform/Infrastructure Engineer

"You're a platform engineer setting up eval infrastructure for multiple teams. You currently use Langfuse for observability."

**Likely answer:** Interested in the OTel observation mode. "Evaluate agents from traces without touching their code" is very appealing. But needs scale, reliability, and integration with existing trace backends (Jaeger, Tempo).

**What would make them switch:** OTel-native, works with existing trace infra, fast (Rust performance), CLI-friendly for automation.

**Switching cost:** Low-medium (additive tool, doesn't replace Langfuse — complements it).

### The User Themselves (Segment 0)

"You're a Rust developer building diverse AI agents. You've tried eval frameworks and found them lacking."

**Likely answer:** Yes, this is exactly what I need. Library-first, Rust-native, flexible, works with my OTel-instrumented agents, doesn't lock me into a framework.

**Beachhead segment:** The user IS the beachhead. Rust AI developers with diverse agent projects and OTel instrumentation. Small segment, but highly motivated and underserved.

---

## Ideas from the Research the User Might Have Missed

### 1. Statistical Multi-Trial as First-Class Feature

Gap 1 from synthesis: "No mainstream tool provides proper statistical treatment of non-deterministic agent evaluation." This maps perfectly to the user's framework:

```
Run::builder()
    .dataset(samples)
    .scorer(task_completion)
    .trials(10)  // run each sample 10 times
    .confidence(0.95)  // 95% confidence interval
    .build()
    .execute()?;

// Result includes:
// - Per-sample: mean score, CI, pass@k, pass^k
// - Aggregate: Wilson CI, Fisher exact test for step-level attribution
// - Drift detection: CUSUM across runs
```

This is the highest-leverage differentiator available. Only Agentrial (16 stars) attempts this. It fits naturally into the framework (a runner feature, not a scorer feature) and requires no new abstractions.

### 2. "Eval Replay" — Re-Evaluate Historical Traces Against New Criteria

No existing tool enables this. But if the framework can evaluate from OTel traces (observe mode) AND trace stores keep historical data, then:

- Write a new scorer
- Point it at historical traces in Jaeger/Tempo
- Evaluate past agent behavior against the new criterion
- See how agent quality would have changed if you'd had this scorer earlier

This is "time-travel evaluation" — retroactive quality assessment. Incredibly powerful for understanding whether a new scoring criterion reveals previously-hidden quality issues.

**Evidence:** Stream 4 (Pattern 4) documents trace-based evaluation. Stream 3 (Job 5) documents the production feedback loop gap. No tool combines these for historical re-evaluation.

### 3. The Framework as an OTel Evaluation Sidecar

Instead of a standalone tool, deploy the eval framework as a sidecar that continuously evaluates traces from a production agent:

- Reads traces from OTel collector or trace store
- Runs configured scorers
- Emits its own OTel spans with evaluation results
- Alerts on quality degradation (via standard OTel alerting)

This is "continuous evaluation" without modifying the agent or its deployment. The sidecar pattern is well-understood in the infrastructure world but hasn't been applied to AI evaluation.

**Evidence:** Stream 3 (Job 5) documents production monitoring gaps. Stream 7 (Trend 3) documents OTel as universal standard. The sidecar pattern from Kubernetes/service mesh world hasn't been applied here.

---

## Generative Ideation

### Idea: "cargo test for evals" — Evaluation as Native Rust Testing

What if the framework integrated with Rust's built-in test framework (`#[test]`, `cargo test`) the way DeepEval integrates with pytest?

```rust
#[eval_test]
fn test_greeting_quality() {
    let sample = Sample::new("Say hello", "Hello! How can I help?");
    let output = my_agent(sample.input());
    assert_score!(exact_match(output, sample.reference()), > 0.8);
}
```

Run with `cargo test` or a custom `cargo eval` command. Results output as structured JSON alongside test output. This gives Rust developers zero-friction adoption — no new tool to learn, just a new test attribute.

**Why the user could do this:** Deep Rust expertise. Building for Rust developers first.
**Why it's not obvious:** Every eval tool builds its own runner. Integrating with the language's native test framework is what DeepEval did for pytest (and it's their most praised feature).

### Idea: Scorer Composition Algebra

Instead of just "run multiple scorers," provide composition operators:

```rust
let quality = coherence.and(relevance).weighted(0.6, 0.4);
let safety = no_pii.and(no_toxicity).all_pass();
let final_score = quality.then(safety); // safety only runs if quality passes
```

This makes complex evaluation logic readable and declarative. The `.and()`, `.weighted()`, `.all_pass()`, `.then()` operators compose scorers into pipelines.

**Why the user could do this:** Rust's trait system and operator overloading make this ergonomic. The generic Scorer trait enables composition naturally.
**Why it's not obvious:** Existing tools compose scorers by running them independently and listing results. No tool provides compositional operators that express evaluation logic as a pipeline.

### Idea: Schema-Validated Scoring

For structured outputs (JSON, YAML), provide a scorer that validates against a schema:

```rust
let scorer = json_schema_scorer(include_str!("excalidraw.schema.json"));
let score = scorer.score(&input, &output, None);
// Returns: Binary(true/false) + detailed validation errors as metadata
```

This addresses the user's Excalidraw use case partially (validate the JSON is valid Excalidraw before rendering to PNG). More broadly, structured output validation is a universal need.

**Evidence:** Stream 5 documents format validation as a common deterministic check. No tool provides schema-based scoring as a first-class feature.
