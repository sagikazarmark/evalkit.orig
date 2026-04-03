# Brainstorm Conversation History

## Context

This conversation followed the braindump process (`braindump.md`) to explore building an AI evaluation framework. The research corpus in `research/` was already complete. The braindump phase produced documents in `braindump/`.

---

## Phase 1 — Braindump Received

**User's raw braindump** covered:

### Core Model
- Evaluation is a simple loop: sample input + expected/reference data → send to agent/actor → get real output → grade/score the output against reference
- Reference data isn't necessarily the "correct output" — it's data the framework can compare/reference against
- Results can be used for: historical comparison (trending), baseline comparison (main branch vs PR, regression gating), absolute threshold checks

### Execution Flexibility
- Most eval tools are opinionated about execution (agent harness, platform, etc.)
- Wants to support: library mode, binary/CLI mode, OTel observation mode, agent harness mode
- Inspired by agentevals-dev: uses OTel spans for evaluation without executing agents — "quite genius"
- Should be low-level enough to support all execution modes

### Scoring/Grading
- Different types: continuous 0-1, pass/fail, labels, metrics (latency, token count)
- Unsure about terminology (scorer, grader, evaluator)
- Unsure about single vs multi-score per scorer
- Comparisons with baselines are challenging due to diverse score types

### Generic Design
- Although primarily for AI evals, the core should be domain-agnostic
- "You have some sample inputs, send it into a blackbox, it spits something out, you grade that output. Notice I haven't used the word AI."

### Tech Stack
- Rust — has existing agent harness and LLM integrations in Rust
- Likes agentevals-dev's pattern: evaluators can be binaries accepting JSON on stdin
- FFI as an option for multi-language (mentioned Restate), but no FFI experience
- Extensibility is important: custom scorers, custom input/output

### Deferred
- Dashboards, CI/CD gating — "eventually"
- Storage format and runs — open question
- Evaluator catalog — not important initially

### OTel
- Should be a run mode, not a hard requirement
- Should it act as OTLP ingest point, use collector, or query trace stores?

### Use Cases
1. Prompt improvement with known-good samples
2. Excalidraw JSON generator → render to PNG → evaluate visually
3. GitHub issue → write failing test → eval runs the test
4. Agent blueprint writer → markdown in OTel spans → evaluate

### Questions Asked
- How does an eval process look exactly (low-level steps)?
- What are execution modes called?
- Scorers? Graders? Evaluators? Correct vocabulary?
- Are there features I didn't think about?

---

## Phase 2 — Interrogation

### Round 1

**Q: Generic framing — design principle or market bet?**
A: Design principle. Keep core abstractions domain-free, layer AI-specific stuff on top.

**Q: Which execution mode would you use first?**
A: Has use cases for all. Easiest to start with is library. Has strong OTel use cases too. Concerned that if not all modes are in the vision, something might get lost in the design.

**Q: Post-processing (transforms) — inside or outside the framework?**
A: Doesn't know. Can see either way. When using library or OTel mode, probably a post-processing step is necessary. Needs research.

**Q: Have you shipped FFI bindings before?**
A: Never did FFI before. It's just an option. Wants Rust, that much is certain.

**Q: "Stable identity" — what do you mean?**
A: Stable identity for sample data: if I send data to the agent, then wait for OTel spans on the other side, how do I know which trace belongs to which sample?

### Round 2

**Q: Correlation — zero code change hard requirement?**
A: Prefers zero code changes. Not sure what agentevals does.

**Q: Existing Rust work — build on it?**
A: Separate work. Wouldn't want to see AI code near the eval API. API should be independent of AI. Implementation can use AI stuff.

**Q: Who is this for besides you?**
A: First user, but expects others will use it.

**Q: Resource situation?**
A: Doesn't care about that concern. Can work on this.

**Q: What "better" means?**
A: Likes high-level solutions with "good defaults" but wants to build from lower-level components. Flexibility is key — works on various different projects with different agents.

### Round 3

**Q: Concrete example of "low-level"?**
A: Scorer = function taking input+output+reference → score (low-level). Pre-built scorer like LLM-as-judge (high-level). Also means the APIs: post-processing may be available but probably not used in many cases.

**Q: What triggered this exploration?**
A: Been trying to use eval frameworks to improve agents, but: they want to lock me in, no good Rust solution, not flexible enough, not well thought out enough.

**Q: Anti-goals confirmed?**
A: Yes — "so complex that I dread writing new evals" and "can't extend without modifying core" are both anti-goals.

**Q: Storage — querying vs reproducing?**
A: Storage can be delayed. Serializable output, caller handles storage. Add storage when patterns emerge.

**Q: Inspirations outside eval space?**
A: Couldn't identify a strong external inspiration.

### Post-Round 3 Clarification

User corrected the "simple loop" framing: the loop is actually two structurally different things:
- **Integrated mode**: framework controls execution (input → run → output → grade)
- **Observation mode**: framework only receives and grades (traces arrive → extract output → grade)

These are fundamentally different — in observation mode, the framework is a receiver and grader, not an orchestrator.

---

## Phase 3 — Record

Structured record saved to `braindump/00-record.md`. User confirmed it looks good.

Key elements extracted:
- 7 Ideas (I-01 through I-07)
- 5 Hypotheses (H-01 through H-05)
- 6 Technical Opinions (T-01 through T-06)
- 8 Questions (Q-01 through Q-08)
- 5 Assumptions (A-01 through A-05)
- Builder profile, anti-goals, resource constraints, evaluation lens

---

## Phase 4 — Cross-Reference

Three parallel research agents were dispatched:

### Research: agentevals-dev Correlation Mechanism
- **WebSocket SDK path**: Requires wrapping agent code in a context manager (NOT zero-code-change). SDK creates session with `session_id`, tags all spans.
- **OTLP HTTP path**: No code changes, but requires setting environment variables (`OTEL_RESOURCE_ATTRIBUTES`). Correlation via `agentevals.session_name`, `agentevals.eval_set_id`.
- **Eval-case matching**: Uses text matching on user content (first user message), not timing.

### Research: Rust Eval Tools
- Near-zero competition: `adk-eval` (1k downloads, tied to framework), `evalframe` (14 downloads, Lua DSL), `tool-eval` (10 downloads, brand new)
- **Restate multi-language approach**: Rust shared core + PyO3 for Python + WASM for TS/Go. NOT FFI (C ABI). WASM as universal portable target, PyO3 as specialized Python bridge.

### Research: OTel Correlation Patterns
- **W3C Trace Context (`traceparent` header) is the best approach**: eval framework sets trace ID, standard OTel HTTP instrumentation automatically propagates it through agent spans. Zero code changes on agent side.
- **OTel Baggage**: Can inject metadata but not automatically added to spans (needs Collector processor or agent-side SpanProcessor).
- **Sequential execution**: Viable fallback (one sample at a time, collect by time window) but slow and fragile.
- **`gen_ai.evaluation.result`**: Not an established event in current OTel GenAI semantic conventions.

### Cross-Reference Results (saved to `braindump/01-cross-reference.md`)

Key findings per idea:
- **I-01 (generic Rust library)**: Genuine gap. No Rust eval framework exists. No generic-core approach exists. Unique and defensible.
- **I-02 (multi-mode)**: No existing tool supports all four modes. Viable if decomposed into acquisition + grading layers.
- **I-03 (OTel observation)**: Viable. traceparent correlation works for HTTP agents. Simpler than agentevals-dev's approach.
- **I-04 (scoring system)**: "Scorer" is the recommended term. Single-score-per-scorer as default, multi-score as opt-in.
- **I-05 (comparison)**: Statistical rigor gap is critical (#1 gap in research). Serializable results enable git-based comparison.
- **I-06 (transforms)**: Not addressed by any existing tool. Optional closure between acquisition and scoring.
- **I-07 (multi-language)**: Restate pattern (PyO3 + WASM) is proven. Binary+JSON complementary for custom scorers.

Hypothesis validation:
- H-01 (generic core): Holds as design principle, needs flexible types (especially `reference: Option<T>`)
- H-02 (existing tools too opinionated): Strongly supported by research
- H-03 (single library all modes): Untested, plausible, needs prototyping
- H-04 (OTel sufficient for many cases): Supported with documented limitations
- H-05 (Rust viable): Partially supported, Python SDK delay is real risk but accepted

---

## Phase 4.5 — Checkpoint

### Top Findings Presented to User

1. agentevals-dev is NOT truly zero-code-change — but traceparent IS zero-code-change for HTTP agents
2. Python SDK is non-negotiable for adoption (research) — Restate proves the path
3. "Reference" is optional in many AI evals — core API needs `reference: Option<T>`
4. Statistical multi-trial evaluation (#1 domain gap) wasn't in braindump — high-leverage differentiator
5. Metrics-only libraries without platforms die — CLI/binary mode is minimum "not just a library"

### Top Opportunities Surfaced

1. Pairwise comparison scoring
2. traceparent-based correlation (simpler than OTLP receiver)
3. Evaluation cost tracking
4. Result serialization as first-class contribution (no standard exists)
5. Restate multi-language pattern (PyO3 + WASM)

### User's Reactions

- **Correlation ID acceptable** — relaxes the zero-code-change assumption. "Some sort of correlation ID should be okay for OTel."
- **Python SDK: delay it** — building for self first, not optimizing for external adoption
- **OTLP**: trace backend query is okay but not always sufficient. OTLP receiver may be needed later.
- **Cost tracking**: "Can we cover this by using tracing+OTel? Instrument the eval framework itself." — clever self-instrumentation insight
- **Pairwise comparison**: "If it doesn't complicate things too much, and is an option, why not?"
- **Multi-language**: delay it

---

## Phase 5 — Discoveries

Saved to `braindump/02-discoveries.md`. Key discoveries:

### Connections Not Made

1. **Self-instrumenting framework** — if the eval framework emits OTel spans for its own operations, cost tracking, performance profiling, AND meta-evaluation all come free. The framework eats its own dog food.

2. **Scorer trait naturally supports pairwise comparison** — `(input, output_a, reference=output_b) → Score`. Pairwise is just a use pattern, not a new abstraction.

3. **Transform step solves "artifact evaluation"** — JSON→PNG, code→compile→test, markdown→render are all instances of `raw_output → transform → artifact → score`. Universal pattern, unformalized by any tool.

4. **Observe mode enables evaluating agents you don't own** — third-party evaluation, historical re-evaluation, cross-team evaluation. More powerful than initially framed.

5. **Serializable results enable "git for eval results"** — commit results alongside code, diff between branches, review in PRs. Zero-infrastructure storage.

### Generative Ideas

1. **"cargo test for evals"** — integrate with Rust's `#[test]` framework via `#[eval_test]` attribute. Zero-friction adoption for Rust developers.
2. **Scorer composition algebra** — `coherence.and(relevance).weighted(0.6, 0.4)` — compositional operators for building complex evaluation logic.
3. **Schema-validated scoring** — JSON Schema validation as a first-class scorer. Addresses structured output validation.

### Layer Decomposition

```
Layer 4: AI-Specific (LLM-as-judge, trajectory scorer, agent harness, AI types)
Layer 3: Execution Modes (CLI, OTel observation, comparison, multi-trial)
Layer 2: Core Framework (Sample, Scorer, Score, Result, Run, inline mode, serialization)
Layer 1: Primitives (Score types, aggregation functions, comparison functions)
```

---

## Phase 6 — Synthesis

Saved to `braindump/synthesis.md`. Key conclusions:

### Ranked Exploration Areas

1. **Core Eval Library** (Rust, generic scorer trait) — foundational, days to build, everything depends on it
2. **OTel Observation Mode** (traceparent correlation, trace backend query) — structural innovation, 1-2 weeks
3. **Statistical Multi-Trial** (confidence intervals, pass@k) — highest-leverage differentiator, days on core
4. **CLI Binary** (subprocess execution, JSONL I/O) — makes tool accessible to non-Rust users
5. **Self-Instrumentation** (OTel for framework internals) — cost tracking, meta-evaluation
6. **Transform Pipeline** (optional output→artifact step) — niche but needed for artifact-producing agents

### Recommended Build Order

1. Prototype Scorer trait (validate API design)
2. Prototype traceparent correlation (validate OTel assumption)
3. Build inline runner + JSONL output
4. Add CLI binary
5. Add multi-trial runner
6. Add OTel observation mode
7. Defer: Python bindings, multi-language, platform, dashboards, CI/CD gating, storage

### Verdict

The positioning is **unique and defensible**. No Rust eval framework exists, no generic-core approach exists, no tool supports all execution modes. Start with the Scorer trait prototype — if `(input, output, reference?) → Score` feels right in Rust, everything else follows.

---

## Files Produced

- `braindump/00-record.md` — structured record of braindump + Q&A
- `braindump/01-cross-reference.md` — every idea cross-referenced against research
- `braindump/02-discoveries.md` — connections, opportunities, generative ideas
- `braindump/synthesis.md` — final deliverable with ranked areas and build order
- `brainstorm/convo.md` — this file (conversation history)
