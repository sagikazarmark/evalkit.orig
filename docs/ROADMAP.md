# evalkit Roadmap

> **Note:** This roadmap remains the canonical long-range plan.
>
> For the updated kernel-boundary decision, near-term sequencing, and verification requirements, see `docs/evalkit-kernel-boundary-plan.md`, which amends this roadmap on that specific topic.

> **Working hypothesis (not a commitment).** A generic, Rust-native evaluation library + toolkit. Library-first; app surface grows only as the library matures. Polyglot via a stable subprocess plugin protocol. OTel-friendly on both sides (ingest + emit). Stay neutral — integrate with observability platforms, don't compete with them. This framing will be re-tested after each phase.
>
> **Explicit non-goals.** A hosted SaaS. A prompt-management product. A full observability UI. Coupling to any single agent framework. Replacing DeepEval/Ragas metric catalogs — adopt their vocabulary, don't fork it.

> **Two backlogs live off the critical path:**
> - [Scorer implementations](./scorers.md) — trait implementations per category.
> - [Integrations](./integrations.md) — `OutputSource`, dataset loader, and exporter implementations.
>
> Work from these backlogs can happen any time an API milestone has unblocked the relevant surface. This roadmap drives *when* an API is ready to absorb them.

---

## Phases

Phases are ordered, not time-boxed. No week estimates. An item is "done" when its exit criteria hold.

- **Phase 0 — API freeze & kernel features.** Blocks everything else.
- **Phase 1 — Polyglot protocol & run-log schema.** Makes evalkit's output and extension points portable.
- **Phase 2 — Streaming / online scoring.** Rust's production-tier advantage.
- **Phase 3 — CI / developer workflow.** PR-diff gates, watch loop.
- **Phase 4 — App surface (`evalkit-server`).** Only what the library can't do alone.
- **Phase 5 — Scale & governance.**

---

## Target workspace shape

Small kernel + focused extension crates. Crate names are provisional; final names decided in Phase 0 step (b).

| Crate | Purpose | Status |
|---|---|---|
| `evalkit` | Kernel: `Sample`, `Dataset`, `Run`, `OutputSource`, `Scorer`, `Score`, `ScorerSet`, `Comparison`, `stats`. Also hosts the run-log schema types and agent/conversation sample shapes until external consumers justify a split. | exists (monolithic) |
| `evalkit-scorers-text` | Deterministic + classical-NLP scorers (Levenshtein, BLEU, ROUGE, retrieval metrics, …) | extract in Phase 0(c) |
| `evalkit-scorers-embed` | Embedding-based scorers. Depends on an embedding provider abstraction. | new |
| `evalkit-scorers-llm` | LLM-as-judge scorers built on **[anyllm](https://github.com/sagikazarmark/anyllm)**. **Does not** re-export the RAG crate; callers import both if needed. | new |
| `evalkit-scorers-rag` | Ragas-compatible scorers (faithfulness, answer-relevancy, context-precision/recall, noise sensitivity). Implementations live here; `-llm` does not re-export. | new |
| `evalkit-scorers-agent` | Trajectory + tool-call scorers. Gated on agent sample shapes landing in the kernel. | new |
| `evalkit-scorers-code` | Sandboxed code execution + AST/type-check scorers. | new, later |
| `evalkit-scorers-multimodal` | Scorers over `evalkit-multimodal` types. | new, later |
| `evalkit-providers` | `OutputSource` impls over anyllm, HTTP, subprocess, trace replay. Absorbs current `evalkit-cli` transport code. | extract + extend |
| `evalkit-multimodal` | Opt-in `Modal { Text, Json, Image, Audio, Bytes }` type + serializers. Not required by the kernel. | new, Phase 1 |
| `evalkit-otel` | OTLP receiver (exists, move) + OTel eval-result span emitter (new, unstable upstream conventions — see Phase 1). | extract + extend |
| `evalkit-exporters-langfuse` | Existing Langfuse exporter. | extract |
| `evalkit-exporters-phoenix` | Phoenix exporter. | new |
| `evalkit-exporters-braintrust` | Braintrust exporter. | new |
| `evalkit-cli` | CLI runner. | exists |
| `evalkit-server` | HTTP API + minimal review UI. | later (Phase 4) |

Umbrella `evalkit` crate re-exports the common 80% via `prelude`.

---

## Phase 0 — API freeze & kernel features

**Goal.** Lock the public API and land the kernel features that every downstream crate, scorer, and integration depends on. Everything we build before this has to be paid for twice.

Phase 0 has **four sub-steps** in order: `(a)` audit → `(b)` decide → `(c)` split → `(d)` features.

### 0(a) — Public API audit

Walk every exported item in `src/lib.rs` and answer three questions per item:

1. Is the name canonical (or at least convergent with DeepEval / Ragas / autoevals / Inspect AI)?
2. Is the ownership model right (owned vs borrowed, generic vs concrete, `Arc` vs plain)?
3. Is the error model right (enum variants, source chains, structured info for callers)?

Concrete items known to need decisions:

- **`Scorer<I, O, R>` default.** Today `R = ()`. Options: keep `R = ()`; promote `Option<R>` into the trait so scorers decide; require explicit opt-in via `ReferenceRequired` marker trait. Decide based on ergonomic fallout.
- **`ScorerContext`.** Does it need `sample_id`, `trial_idx`, `run_id`, `metadata: HashMap<String, Value>`? Scorers that emit traces or call judges will want these. Add before the split.
- **`Score` enum.** Add a `Structured` variant? Three candidate shapes:
  - `Structured(serde_json::Value)` — fully free-form, cheap.
  - `Structured { score: f64, reasoning: String, metadata: Value }` — explicit slots for the LLM-judge case.
  - `Reasoned(Box<Score>, String)` — wrap any existing variant with reasoning.
  Pick one; hold the others as rejected alternatives in the decisions log.
- **`OutputSourceError` variants.** Freeze them now. They propagate through every pipeline.
- **`ScorerError`.** Currently a newtype around `Box<dyn Error>`. Consider richer variants (`Timeout`, `ProviderError`, `InvalidInput`, `Internal`).
- **`Run` vs `Executor`.** Does `Run` stay as-is for batch, and a separate `Executor` trait appear later for streaming? Or does `Run` grow? Decision-deferred items noted here so Phase 2 can act.
- **`Score::Metric`** — does `unit: Option<String>` carry enough for cost-tracking, or do we need a structured `Unit` enum?

Deliverable: `docs/decisions.md` capturing each decision + one rejected alternative.

### 0(b) — Decisions, semver policy, multi-crate layout

- Commit decisions from 0(a).
- Write `docs/stability.md`: what semver guarantees apply to `evalkit`, to the extension crates, and to the run-log schema (schema versions independent of crate versions).
- Lock the workspace layout (see table above) before the split actually happens.

### 0(c) — Workspace split

Execute the split. Each move is one PR, API-compatible where possible.

1. Extract `evalkit-scorers-text` (current deterministic scorers in `src/scorers/`).
2. Extract `evalkit-providers` (HTTP + subprocess sources currently inside `evalkit-cli`).
3. Extract `evalkit-otel` (`src/otel.rs` + `OtlpReceiver` + `JaegerBackend`).
4. Extract `evalkit-exporters-langfuse` (`src/langfuse.rs`).
5. Create empty skeletons: `evalkit-scorers-llm`, `evalkit-scorers-rag`, `evalkit-scorers-embed`. Leave agent/code/multimodal crates uncreated until their sample shapes exist.

Kernel ends Phase 0 dependency-light. The existing `default-features = []`, `otel`, `llm-judge`, `langfuse` feature flags collapse into per-crate opt-ins.

### 0(d) — Kernel features

These require the API to be frozen; hence they come after (a)–(c).

- **`RunMetadata`.** Populate with dataset hash, code/commit, judge model pins, timestamp, seed, scorer config fingerprint. Goal: two runs with identical metadata produce identical results.
- **Deterministic seeding.** `Run::builder()` accepts an optional RNG seed; threaded through trial ordering and any scorer randomness.
- **Cost / token tracking.** Add `TokenUsage { input: u64, output: u64, cache_read: u64, cache_write: u64 }` and `cost_usd: Option<f64>` to `SampleResult`. Populate from source + scorer spans.
- **Per-sample, per-scorer timing.** `Duration` fields on `TrialResult`.
- **Statistical rigor in `stats.rs` / `Comparison`.**
  - Wilson confidence intervals for binary scores.
  - Bootstrap CIs for numeric scores.
  - Paired significance tests (t-test or Wilcoxon) in `Comparison`.
- **Scorer composition operators** (kernel API additions, not scorer impls):
  - `.or()` — any-pass.
  - `.not()` — invert `Score::Binary`.
  - `.map_score(f)` — post-transform.
  - `.timeout(d)` — bound scorer latency, produce explicit timeout error.
- **`Score::Structured`.** Land whichever variant shape 0(a) chose.

**Exit criteria for Phase 0:**
- `evalkit` 0.2.0 released with the frozen kernel API.
- Workspace split complete; extension crates compile and publish independently.
- `docs/decisions.md` and `docs/stability.md` in-repo.
- Every downstream scorer or integration starts from known-stable traits.

---

## Phase 1 — Polyglot protocol & run-log schema

**Goal.** Make evalkit's output and extension points portable, so polyglot users (Python, TS) and other tools can cooperate.

### 1(a) — Run-log schema (as a kernel module first)

- JSON Schema v1 for `RunResult`, `SampleResult`, `TrialResult`, `Score`, `RunMetadata`. Lives in `evalkit::schema` (not a separate crate — extract only when an external consumer needs it).
- `write_jsonl` / `read_jsonl` output conforms to schema v1 byte-for-byte; schema version recorded in the header line.
- Breaking changes bump the schema major version; older readers fail loud.
- Publish the JSON Schema document in-repo so other tools can target it.

### 1(b) — Subprocess plugin protocol (formal spec)

Promote the current `evalkit-cli` stdin/stdout JSON-line convention into a stable spec.

- Document: `docs/plugin-protocol.md`.
- Two plugin kinds: **`OutputSource` plugin** and **`Scorer` plugin**.
- Versioned handshake: plugin declares `{ kind, name, version, schema_version, capabilities }`.
- Error model: plugin errors map to `OutputSourceError::ExecutionFailed` / `ScorerError` with the plugin's error payload preserved.
- Reference shims (thin wrappers — a decorator plus a stdio loop):
  - `evalkit-plugin` on pypi.
  - `@evalkit/plugin` on npm.
- Conformance suite: a fixture-driven harness in `evalkit-providers` that validates any plugin binary against the spec.

### 1(c) — anyllm-backed LLM-judge primitive

> **Prerequisite.** Read anyllm's source and confirm the exact trait name and chat method signature. The pseudocode below is illustrative; concrete shape resolves after inspection. File the findings in `docs/decisions.md`.

Illustrative target shape (names subject to anyllm's actual API):

```rust
pub struct LlmJudge</* anyllm provider handle */> {
    provider: /* anyllm provider */,
    model: String,
    prompt: PromptTemplate,              // canonical-form normalized for hashing
    extractor: ScoreExtractor,           // Binary | Numeric | Label | Structured
    retries: usize,
    timeout: Duration,
    temperature: f32,                    // default 0.0 for determinism
    capture_reasoning: bool,
}
```

Requirements:

- Uses anyllm's **structured output** (JSON schema / function-calling) for score extraction. Never parse freeform text for the score itself.
- If `capture_reasoning = true`, produce `Score::Structured { … reasoning … }` (shape per the Phase-0 decision).
- `PromptTemplate` has a canonical string form (whitespace/ordering normalized) so `prompt_hash` is stable. Ship a `prompt_hash()` helper; otherwise the hash is unreliable and not worth recording.
- `LlmJudgeEnsemble`: N judges, aggregation (majority for binary, mean/median for numeric).
- `LlmJudgeCalibrator`: pass `(sample, gold_score)` pairs, report agreement statistics against the judge.
- Populate `TokenUsage` and `cost_usd` on the scorer's metadata.

### 1(d) — Agent + multi-turn sample shapes

Add concrete sample types in the kernel (modules, not crates) so `evalkit-scorers-agent` can start:

- `TrajectorySample<I, R>`: ordered `Step { role, content, tool_calls, tool_results }` sequence.
- `ConversationSample<I, R>`: turns with stable IDs, so multi-turn scorers can reference specific turns.
- `ToolCall { name, arguments, call_id }` struct used by both.

Keep the kernel generic: `Scorer<I, O, R>` still accepts any `O`, and these are concrete `O` shapes users may choose.

### 1(e) — OTel eval-result emission (tracking upstream)

> **Caveat.** OTel GenAI semantic conventions are in-flight; eval-result conventions are more nascent still. Do not treat this as a stable target. Ship an interim shape explicitly versioned by evalkit; revisit when upstream conventions land.

- Emit eval results as OTel spans via `evalkit-otel::OtelResultEmitter`.
- One span per `Run`, child spans per `SampleResult`, events per `Score`.
- Attribute names follow our own `evalkit.*` namespace until GenAI eval conventions stabilize; map to upstream once they do.
- Works as a `RunResult` post-processor and as a streaming hook (Phase 2).
- Benefits Langfuse / Phoenix / any OTel backend for free.

**Exit criteria for Phase 1:**
- Schema v1 frozen and documented.
- Plugin spec + reference shims published.
- anyllm-backed `LlmJudge` primitive shipping with at least `llm_judge`, `g_eval`, and `llm_classifier`.
- `TrajectorySample` and `ConversationSample` in the kernel.
- Interim OTel emission available behind a feature flag, versioned.

At this point the scorer backlog in `docs/scorers.md` can be mass-executed — that's mechanical work from here on.

---

## Phase 2 — Streaming / online scoring

**Goal.** Rust's natural advantage — the production worker tier.

- `Executor` trait (decision from Phase 0 determines exact shape): pull samples from a source, produce output async, score async, push results. Backpressure, bounded queues, graceful shutdown.
- Sampling strategies: `PercentSampler`, `TargetedSampler` (predicate-based), `AlwaysSampler`.
- **Judge-model tiering**: pipeline where a cheap scorer flags, then an expensive scorer re-scores the flagged subset.
- Partial / streaming scoring: call a scorer on an incomplete output (token-by-token streaming cases).
- Source adapters: `OtlpReceiver` (exists) → `Executor` input; Kafka; NATS; file tailer.
- `evalkit-otel::OtelResultEmitter` (from Phase 1) used as the default sink.

**Exit criteria for Phase 2:**
- A "prod-eval daemon" is a ~200-LOC example binary composed from library primitives — proves the kernel is production-grade without needing the `evalkit-server` app tier.

---

## Phase 3 — CI / developer workflow

**Goal.** Make evalkit the obvious pick for eval-gates-on-PR in any Rust-or-polyglot shop.

- `evalkit diff <run-a> <run-b>` CLI: calls the kernel's existing `compare` and emits markdown + JSON suitable for a PR comment.
- `evalkit watch`: iterate-on-prompt loop; re-runs on file change. Useful for local iteration, not CI.
- **GitHub Action (single home).** The action wraps `evalkit run` + `evalkit diff`, posts the markdown diff as a PR comment, honors `threshold` config in the CLI. All PR-comment logic lives here — not duplicated in an exporter.
- Formalize the CLI's TOML config spec into `docs/cli-config.md`.
- Dataset splits / tags / filters in the runner.

---

## Phase 4 — App surface (`evalkit-server`)

Only what the library genuinely can't do alone.

- HTTP API over a SQLite run store: list / get / diff / annotate.
- Minimal web UI: browse runs, drill into failed samples, visual diff between two runs.
- Annotation queue → promote annotated samples to a dataset file (closes the production-feedback-loop gap called out in prior research).
- Prod-eval dashboard: OTLP in, scored samples out, threshold alerts. Built on Phase 2 primitives.
- Everything optional; the library remains usable without it.

---

## Phase 5 — Scale & governance

- Distributed run sharding.
- PII scrubbing hooks in the eval pipeline.
- Drift detection on streaming eval results.
- Red-team / adversarial scorer packs (likely their own crate).

---

## LLM-judge scorers (using anyllm)

anyllm gives us provider-neutral chat + structured output + streaming + tool-calling across OpenAI / Anthropic / Gemini / OpenAI-compatible / Cloudflare. This section lists *what* to ship; *how* (the `LlmJudge` primitive) is specified in Phase 1(c).

Full tick-list lives in [`docs/scorers.md`](./scorers.md); priorities below.

### First cohort (Phase 1, ships with the primitive)
- `llm_judge` — freeform rubric (port existing impl to anyllm)
- `g_eval` — chain-of-thought rubric; auto-generate eval steps from criteria. Highest adoption driver in DeepEval; ship early.
- `llm_classifier` — N-class classification

### Correctness / factuality
- `model_graded_qa`, `model_graded_fact`, `factuality`, `closed_qa`, `answer_correctness`, `answer_similarity_llm`

### RAG (in `evalkit-scorers-rag`; not re-exported from `-llm`)
- `faithfulness` (alias: `groundedness`), `answer_relevancy`, `context_precision`, `context_recall`, `context_entity_recall`, `noise_sensitivity`, `hallucination`
- Paired with deterministic retrieval metrics in `evalkit-scorers-text`: `precision_at_k`, `recall_at_k`, `mrr`, `ndcg`

### Pairwise / ranking
- `battle`, `pairwise_preference`, `select_best`, `select_worst`, `ranking`

### Quality / form
- `summarization_quality`, `conciseness`, `verbosity`, `translation_quality`, `sql_semantic_equivalence`, `code_semantic_equivalence`
- `instruction_following` (IFEval-style)

### Safety / governance
- `toxicity`, `bias`, `pii_leakage`, `refusal_appropriateness`, `misuse`, `policy_adherence`, `jailbreak_detected`

### Agent (after `TrajectorySample` lands)
- `trajectory_judge`, `tool_call_appropriateness`, `plan_quality`, `plan_adherence`, `sub_goal_completion`, `step_efficiency`

### Conversational (after `ConversationSample` lands)
- `conversation_coherence`, `knowledge_retention`, `role_adherence`, `conversation_completeness`

### Cross-cutting scorer DX
- **Prompt templates** live under `evalkit-scorers-llm/prompts/`, one file per scorer, overridable via `LlmJudge::with_prompt`. Canonical-form normalization (Phase 1(c)) makes `prompt_hash` stable.
- Temperature `0.0` default; pass seeds where providers support them.
- Structured output only; never parse freeform text for the score.
- `definition()` returns rich metadata: `name`, `direction`, `judge_model`, `prompt_hash`, `provider_id`.
- Note on **`pass_at_k`**: this is a **trial-aggregate**, not a per-sample scorer. Implement in `stats.rs` or `evalkit-scorers-code`, not in `-llm`.

---

## Deferred

Moved out of Phases 0–2 to keep focus. Not dropped — tracked in backlog.

- **Persistence layer (SQLite run store).** Phase 4 / `evalkit-server`.
- **Web UI.** Phase 4.
- **Distributed execution.** Phase 5.
- **Multi-modal scorers.** `evalkit-scorers-multimodal`, after `evalkit-multimodal` types land in Phase 1+.
- **Red-team / adversarial scorer pack.** Phase 5.
- **Public benchmark adapters** (MMLU, HumanEval, SWE-bench, …). Lazy — add when a user needs them.

---

## Open questions to resolve during Phase 0

1. Which `Score::Structured` shape wins (see 0(a))?
2. Does `ScorerContext` absorb metadata fields, or do they go on `TrialResult` only?
3. Does anyllm expose a single `Provider` trait, or do judge scorers accept each provider as a distinct type parameter?
4. Is `Run` extended for streaming, or does a separate `Executor` trait appear in Phase 2?
5. Where does a composite `Unit` type live for `Score::Metric` (stays `Option<String>` or becomes structured)?

Answers go in `docs/decisions.md` with one rejected alternative each, so future-us can understand why the path not taken was rejected.

---

## Meta

- The two backlog docs ([`scorers.md`](./scorers.md), [`integrations.md`](./integrations.md)) are append-only lists — triage in, tick off as shipped.
- Phase 0 items are blockers. Phase 1 items are parallelizable once 0 is done.
- Revisit this roadmap after each phase exit. The vision framing at the top is a working hypothesis, not a commitment.
