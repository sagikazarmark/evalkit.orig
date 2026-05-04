# Post-2.0 Direction Notes

**Date:** 2026-05-02
**Context:** This document captures the design conversations that followed the evalkit 2.0 release (HEAD `e6e0402`). It complements existing docs without superseding them — see `docs/ROADMAP.md` for the long-range plan, `docs/gap-analysis.md` for the canonical pre-2.0 inventory, and `docs/competitive-analysis-2026-04.md` for the framework comparison.

The point of this doc: capture what the 2.0 work taught us about where to go next, so future direction decisions don't have to re-derive it.

---

## 1. What 2.0 shipped

The bundled kernel output API redesign. See `CHANGELOG.md` and `docs/superpowers/specs/2026-04-27-kernel-output-api-2.0-design.md` for the full list. Highlights:

- `OutputSource::produce` returns `ProductionOutput<O>` with optional `usage`, `cost_usd`, `latency`, `metadata`. Source-side cost is now expressible.
- `Score::Structured` removed. Reasoning lives on `ScoreOutcome` and `ScoredEntry` next to the score, not inside it. Label and binary judges newly support reasoning.
- `ScorerContext` gains `seed`, `cancel` (tokio-util `CancellationToken`), `budget` (advisory `Budget`), `previous_scores` (sibling-score visibility within a `ScorerSet`).
- `ScorerResources` → `ResourceUsage` with new `latency` field.
- `SampleResult` splits `source_resources` / `scorer_resources`.
- Snapshots moved to `evalkit-runtime` as a `SnapshotSource` extension trait.
- OTel-specific concerns moved to `evalkit-otel`.
- Schema bump `"2"` → `"3"`.
- `evalkit migrate-runlog` CLI subcommand for v2 → v3.

29 commits, 30 tasks, all workspace tests green. Tagged release pending.

---

## 2. Real-world eval use-case coverage (user-facing lens)

This is the framing that matters for product decisions: "what do people actually use eval frameworks for, and how does post-2.0 evalkit cover those?"

| Use case | Coverage post-2.0 | Status |
|---|---|---|
| **RAG / customer-support chatbot** (faithfulness, answer relevancy, retrieval@k) | `evalkit-scorers-rag` is empty; `evalkit-scorers-text` lacks retrieval metrics | **Largest gap** |
| **Agent / tool-use eval** (trajectory, tool-call appropriateness) | `TrajectorySample` / `ToolCall` shapes exist in kernel; no `evalkit-scorers-agent` crate | Shapes ready, scorers missing |
| **Coding-agent eval** (SWE-bench, pass@k, regression rate) | Blocked: no sandbox, no MCP plumbing, no diff/test scorer (per `docs/code-agent-eval-suite-spec.md`) | Blocked on infrastructure |
| **Red-team / safety** (jailbreak, PII leakage, prompt injection) | Heuristic regex scorers in `evalkit-scorers-redteam`; no adversarial corpus or multi-turn simulator | Shallow |
| **CI eval gates on PR** (threshold, diff comment) | `evalkit diff` + GitHub Action source-only; cost/latency thresholds NOT gates | Mostly there |
| **Continuous / production monitoring** (sample %, drift, judge tiering) | `PullExecutor` + samplers + `OtelResultSink` + drift detection ✓ | Strongest area |
| **Pairwise / model-comparison** (`battle`, `pairwise_preference`) | `Comparison` + paired-bootstrap stats ✓; pairwise scorers not shipped | Half there |
| **Multi-modal eval** (image, audio, video) | Not started; `evalkit-multimodal` was Phase 1 plan | Not started |
| **Cost / token attribution** (per-user, per-feature) | `TokenUsage` + `cost_usd` captured per-side post-2.0 ✓; tag-based grouping and budget gates missing | Partial |

**Highest-leverage gap by far: RAG.** RAG is the most common production use case across the framework comparison; the 2.0 envelope unblocks it (judges with reasoning, source-side metadata for retrieved contexts), and the empty `evalkit-scorers-rag` crate is the natural starting point.

---

## 3. Industry framework comparison highlights

From multi-agent research conducted this session. Cited extensively in metric-rearchitecture exploration; preserved here for future reference.

**On scoring vs measurement / metrics:**

- **Inspect AI** (UK AISI — closest peer for LLM eval domain): `Score` carries the value (literal/float/int/bool), `Metric` aggregates over scores. Producer-declares reductions. `ModelOutput.usage` and `time` are typed fields on the model output — separate from scoring.
- **Promptfoo:** assertions return pass/fail + score; `tokenUsage`, `cost`, `cached` are separate columns on the result row.
- **DeepEval:** "Metric" *is* the scorer in DeepEval — naming overload that affects evalkit's vocabulary choices. `LLMTestCase` has optional `token_cost` and `completion_time` slots.
- **Braintrust:** clean two-word split — `scores` for quality, `metrics` for performance/usage. `span.log({metrics: {...}})` is opt-in via spans.
- **LangSmith:** evaluator returns `EvaluationResult`; tokens/cost/latency auto-captured on the run itself, not via evaluators.
- **OpenAI Evals:** scores only; no measurement concept at the protocol level.
- **MLflow / W&B:** generic experiment tracking — single flat metric channel, consumer picks aggregation at query time.
- **Prometheus / OpenTelemetry:** producer-declared metric *type* drives correctness (`rate()` only valid on counters; histogram bucketing math). Type is a correctness contract, not a UI hint.

**Naming convention winner: "metric"** (MLflow, W&B, Promptfoo, DeepEval, Braintrust, LangSmith, Prometheus, OTel, Datadog all use it).

**Aggregation-strategy split by domain:** production observability requires producer-declared types (correctness depends on it). Generic experiment tracking goes consumer-side (no correctness hazard). Inspect AI — evalkit's closest peer — goes **producer-declared**. Eval metrics carry semantic intent: `accuracy` is mean-of-booleans, `stderr` needs sample variance.

---

## 4. Kernel stability + flexibility recommendations

These came out of the design conversations on what to do *after* 2.0. They're separate from the metric rearchitecture (`docs/superpowers/specs/2026-05-02-metric-rearchitecture-exploration.md`); none of them are blocked on that decision.

**Locked direction (user endorsed):**

1. **Type-level binding between `Scorer` and the `Score` variant it returns** — eliminates the `Mixed` accumulator footgun at compile time. Mid-sized refactor (associated type or marker trait).
2. **`ScoreDefinition` carries bounds + units + range** — eliminates the "47.0 in a [0,1] scorer" footgun via runtime validation in `validate_score`. Additive.
3. **Property tests for serde round-trips and arithmetic** — `Score`, `RunResult`, `ResourceUsage::merge`, `transform_score_entry`. Catches regressions unit tests miss.
4. **MIRI for the `unsafe` blocks in `MappedRunExecutor`** — type-state-justified; CI job to keep them honest.
5. **Comprehensive rustdoc with `#![deny(missing_docs)]`** — locks in semantics now, while the design is fresh.
6. **`RunExecutor` promoted to public trait** — users can plug in custom strategies (parallel trials, distributed) without forking. Single biggest flexibility unlock.
7. **`OutputSourceError::is_retryable()`** — already shipped in 2.0.

**Open / deferred:**

- Async `Mapper` (or async variant)
- `RunMetadata.user_metadata: HashMap<String, Value>` for experiment-tracking IDs
- `Send + Sync` constraint on `OutputSource` / `Scorer` — single-threaded variants?
- `Dataset<I, R>` as a trait (streaming, JSONL-backed, computed)

If you're picking next work and don't want a metric-rearchitecture rabbit hole: **#6 (public `RunExecutor`)** and **#1 (type-level Scorer/Score binding)** have the highest ratio of leverage to risk.

---

## 5. Open architectural decisions

In rough priority order:

1. **`Score::Metric` removal** — pending decision (Q2 in the metric exploration). 24 references across 8 files; mechanical to remove if approved. Not blocked on the broader metric rearchitecture — could be done independently.
2. **`Aggregator` declaration site** — producer-declared (Inspect AI / OTel / Datadog precedent) vs consumer-picked (MLflow / W&B precedent). The cross-framework research landed strongly on producer-declared for evalkit's domain.
3. **`ProductionOutput` typed fields** — full collapse into `metrics` channel vs surgical collapse (replace at producer boundary; preserve `ResourceUsage` typed at aggregation boundary). Agent review landed on surgical.
4. **Stability improvements above** — order of attack.

---

## 6. Deferrals from 2.0 (carried forward)

Items that were intentionally deferred during the 2.0 work:

1. **HTTP / subprocess plugins surfacing usage / cost / latency** — needs plugin-protocol bump. Capture into `metrics` channel once plugin protocol v2 is designed.
2. **Full v3 JSON schema document** — only the schema_version constant was bumped; `TrialResult` / `SampleResult` shape definitions in `docs/schema/run-log-v3.schema.json` are partially synchronized. Embedded note in the JSON file.
3. **`current_sample_id` task-local stays in kernel** — moving it to `evalkit-otel` would invert the `evalkit-runtime` / `evalkit-otel` layering. Future fix: `OtelObserver` derives sample id from input or move task-local to `evalkit-runtime`.
4. **Per-task review minor comments** — captured in agent reports during 2.0 implementation, not actioned. Examples:
   - `panic_message` helper duplication between `evalkit/src/run.rs` and `evalkit-runtime/src/lib.rs`
   - Reference-side SAFETY comment expansion in `MappedRunExecutor`
   - `with_current_sample_id` missing doc comment
   - `OtelTraceNotFound` `Clone` derive justification
   - `migrate_storage_v2_to_v3` idempotency on double-runs

---

## 7. Pointers to artifacts from this session

- **2.0 spec:** `docs/superpowers/specs/2026-04-27-kernel-output-api-2.0-design.md`
- **2.0 plan:** `docs/superpowers/plans/2026-04-27-kernel-output-api-2.0.md`
- **2.0 changelog:** `CHANGELOG.md`
- **Metric rearchitecture exploration:** `docs/superpowers/specs/2026-05-02-metric-rearchitecture-exploration.md`
- **2.0 commits:** `git log 3f13ef4..e6e0402`
- **Tag (pending):** `v2.0.0` not yet pushed

---

## 8. Working preferences observed this session

These are observations about how this project operates that future sessions can rely on:

- **Breaking changes are fine for kernel evolution.** "I don't care about breaking changes" stated explicitly twice. Schema bumps are acceptable; migration tools (e.g., `evalkit migrate-runlog`, `evalkit_server::migrate_storage_v2_to_v3`) compensate.
- **Multi-agent pressure tests are valuable.** Both rounds in this session caught real errors (industry-naming, `Score::Metric` status, aggregation strategy). Default to dispatching agents for non-trivial design decisions before committing.
- **"Stability + flexibility for the core" outranks new feature scope.** When given the choice, the user picked making the core sharper over expanding feature surface.
- **Industry comparison matters more than novelty.** Aligning with peer naming and patterns (Braintrust split, Inspect AI conventions) was repeatedly preferred over inventing new vocabulary.
- **Brainstorming before designing.** The brainstorming skill is invoked deliberately; don't skip the explore-options step.
