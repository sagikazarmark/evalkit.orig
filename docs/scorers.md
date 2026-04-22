# Scorer Backlog

> Implementations of `Scorer<I, O, R>`. Most are mechanical once the kernel API is stable. Group by crate.
> Status legend: `[x]` shipped · `[~]` partial · `[ ]` todo.
>
> Naming follows convergent vocabulary across DeepEval / Ragas / autoevals / Inspect AI / Promptfoo — do not invent names when a canonical one exists.

---

## `evalkit` (kernel — keep minimal)

Only the scorers needed by the kernel's examples + tests. Everything else moves out.

- [x] `exact_match`
- [x] `contains`
- [x] `regex`
- [x] `json_schema`

Composition operators (in `scorer_ext`) — shipped.
- [x] `.and()` — `AndScorer`
- [x] `.or()` — `OrScorer`
- [x] `.not()` — `NotScorer`
- [x] `.map_score()` — `MapScoreScorer`
- [x] `.timeout()` — `TimeoutScorer`
- [x] `.weighted()` — `WeightedScorer`
- [x] `.then()` — `ThenScorer`
- [x] `ignore_reference`

---

## `evalkit-scorers-text` (deterministic + classical NLP)

### String / substring
- [ ] `starts_with`
- [ ] `ends_with`
- [ ] `icontains` (case-insensitive)
- [ ] `iexact_match`
- [ ] `trimmed_exact_match`
- [ ] `normalized_whitespace_match`
- [ ] `line_count_equals`
- [ ] `word_count_within_range`

### Structure / validation
- [ ] `is_json`
- [ ] `is_valid_sql` (via sqlparser-rs)
- [ ] `is_valid_xml`
- [ ] `is_valid_yaml`
- [ ] `is_valid_toml`
- [ ] `is_valid_url`
- [ ] `is_valid_email`
- [ ] `is_valid_uuid`
- [ ] `is_valid_iso_timestamp`
- [ ] `is_valid_regex`
- [ ] `json_subset` — output JSON is a structural subset of reference
- [ ] `json_diff_count` — number of differing paths

### Numeric / math
- [ ] `numeric_equals` (exact)
- [ ] `numeric_close` (within epsilon)
- [ ] `numeric_relative_close` (within relative tolerance)
- [ ] `math_expression_equivalent` — SymPy-style; may need subprocess
- [ ] `percentage_within_range`

### Edit distance / fuzzy
- [ ] `levenshtein`            — raw distance, `Score::Numeric`
- [ ] `levenshtein_ratio`      — normalized 0..1
- [ ] `damerau_levenshtein`
- [ ] `hamming`
- [ ] `jaro_winkler`
- [ ] `fuzzy_match`            — threshold-binary over Levenshtein ratio

### Classical NLP
- [ ] `bleu` (1–4-gram, corpus + sentence)
- [ ] `rouge_1`, `rouge_2`, `rouge_l`, `rouge_lsum`
- [ ] `meteor`
- [ ] `chrf` / `chrf++`
- [ ] `gleu`
- [ ] `ter`
- [ ] `word_overlap_f1`
- [ ] `character_f1`
- [ ] `token_f1`
- [ ] `exact_answer_match` (normalize punct/articles — SQuAD-style)

### Retrieval (deterministic — no LLM)
For RAG pipelines scored against a ground-truth relevant-doc set.
- [ ] `precision_at_k`
- [ ] `recall_at_k`
- [ ] `mrr`                    — mean reciprocal rank
- [ ] `ndcg`                   — normalized discounted cumulative gain
- [ ] `hit_at_k`

### Instruction-following (deterministic)
- [ ] `instruction_following`  — IFEval-style verifiable instructions (regex + length + format checks)

### Heuristic / infra
- [ ] `latency_under`          — checks acquisition latency
- [ ] `cost_under`             — checks cost
- [ ] `token_count`            — returns `Score::Metric`
- [ ] `length_ratio`           — output len / reference len
- [ ] `is_refusal_heuristic`   — regex-based refusal detector (fast; complement to LLM refusal judge)
- [ ] `pii_regex`              — pattern-based PII sniff

### Classification aggregates
- [ ] `label_accuracy`
- [ ] `label_precision`
- [ ] `label_recall`
- [ ] `label_f1`
- [ ] `confusion_matrix`       — emits `Score::Structured`

These are run-level aggregates — may live in `stats.rs` instead of as scorers.

---

## `evalkit-scorers-embed` (embedding-based)

Depends on an embedding provider abstraction (anyllm exposes embeddings for OpenAI / Gemini).

- [ ] `embedding_cosine`
- [ ] `embedding_dot`
- [ ] `embedding_euclidean`
- [ ] `semantic_similarity` — threshold-binary over cosine
- [ ] `answer_similarity_embed` — embedding version of Ragas answer_similarity
- [ ] `bertscore` (precision / recall / f1) — needs contextual embeddings
- [ ] `embedding_nearest_label` — classify via nearest embedded label

---

## `evalkit-scorers-llm` (LLM-as-judge; built on anyllm)

See [ROADMAP.md § LLM-judge scorers](./ROADMAP.md#llm-judge-scorers-using-anyllm) for the design.

### Generic rubric
- [~] `llm_judge`         — port existing impl to anyllm + structured output
- [ ] `g_eval`            — chain-of-thought rubric; auto-generate eval steps from criteria
- [ ] `llm_classifier`    — N-class classification
- [ ] `rubric_dag`        — DeepEval DAG-style multi-step rubric

### Correctness / factuality
- [ ] `model_graded_qa`
- [ ] `model_graded_fact`
- [ ] `factuality`
- [ ] `closed_qa`
- [ ] `answer_correctness`
- [ ] `answer_similarity_llm`

### RAG
RAG judge scorers live in `evalkit-scorers-rag`. `evalkit-scorers-llm` does **not** re-export them — users who need RAG import both crates explicitly. This keeps crate dependencies directional.

### Pairwise / ranking
- [ ] `battle`
- [ ] `pairwise_preference`
- [ ] `select_best`
- [ ] `select_worst`
- [ ] `ranking`

### Quality / form
- [ ] `summarization_quality`
- [ ] `conciseness`
- [ ] `verbosity`
- [ ] `humor`
- [ ] `translation_quality`
- [ ] `sql_semantic_equivalence`
- [ ] `code_semantic_equivalence`

### Safety / governance
- [ ] `toxicity`
- [ ] `bias`
- [ ] `pii_leakage`
- [ ] `refusal_appropriateness`
- [ ] `misuse`
- [ ] `policy_adherence`
- [ ] `jailbreak_detected`

### Infrastructure
- [ ] `LlmJudgeEnsemble` — N judges, aggregate
- [ ] `LlmJudgeCalibrator` — helper to measure agreement with gold labels
- [ ] `LlmJudgeCache` — content-addressed cache for judge calls

---

## `evalkit-scorers-rag` (Ragas-compatible)

Canonical home of the RAG judge scorers. Depends on `evalkit-scorers-llm` for the `LlmJudge` primitive. Requires a `RagSample` shape: `input`, `retrieved_contexts: Vec<String>`, `reference`, `output`.

- [ ] `faithfulness` (alias: `groundedness` — ship both names)
- [ ] `answer_relevancy`
- [ ] `context_precision`
- [ ] `context_recall`
- [ ] `context_entity_recall`
- [ ] `noise_sensitivity`
- [ ] `hallucination`
- [ ] `answer_correctness`
- [ ] `answer_similarity`

---

## `evalkit-scorers-agent` (trajectory + tool-call)

Depends on `TrajectorySample` landing in the kernel (Phase 1).

### Trajectory match (deterministic, agent-evals taxonomy)
- [ ] `trajectory_exact_match`
- [ ] `trajectory_unordered_match`
- [ ] `trajectory_subset_match`
- [ ] `trajectory_superset_match`

### Tool call
- [ ] `tool_call_exact`                — strict name + args
- [ ] `tool_call_name_match`           — name only
- [ ] `tool_call_args_match`           — args subset / equality
- [ ] `tool_call_custom_matcher`       — user-provided matcher fn
- [ ] `tool_selection_f1`              — set-based F1 over called tools
- [ ] `tool_selection_node_f1`         — node F1 (agent-evals metric)
- [ ] `tool_order_edit_distance`       — normalized edit distance on call sequence

### Step / plan (LLM-judge-backed)
- [ ] `tool_call_appropriateness`
- [ ] `plan_quality`
- [ ] `plan_adherence`
- [ ] `sub_goal_completion`
- [ ] `step_efficiency`
- [ ] `trajectory_judge`

### Agent-level heuristics
- [ ] `step_count_within_range`
- [ ] `terminated_cleanly`

---

## `evalkit-scorers-code` (code generation)

- [ ] `code_executes_without_error` — subprocess sandbox
- [ ] `code_passes_test_cases`      — pytest / cargo test in sandbox
- [ ] `code_ast_equivalent`
- [ ] `code_type_checks`            — language-specific
- [ ] `code_compiles`
- [ ] `humaneval_compatible`        — convention wrapper

> **`pass_at_k` is a trial-aggregate**, not a per-sample scorer. It belongs in `stats.rs` or as a `RunResult` post-processor — not as a `Scorer` impl. Listed here only as a reminder.

---

## `evalkit-scorers-conversation`

Depends on `ConversationSample` landing (Phase 1).

- [ ] `conversation_coherence`
- [ ] `knowledge_retention`
- [ ] `role_adherence`
- [ ] `conversation_completeness`
- [ ] `turn_count_within_range`
- [ ] `user_satisfaction_judge`

---

## `evalkit-scorers-multimodal` (opt-in)

Depends on `evalkit-multimodal` landing.

- [ ] `image_caption_match`         — LLM-judge over vision model
- [ ] `image_similarity`            — embedding-based
- [ ] `ocr_exact_match`             — OCR output vs reference
- [ ] `audio_transcript_match`      — transcribe via provider, then text scorer
- [ ] `audio_wer`                   — word error rate

---

## Cross-cutting requirements for every scorer

- Declarative metadata (`ScoreDefinition`): name, direction, category, whether LLM-backed, required context fields.
- Serde round-trip for the scorer's config (for run manifests).
- `Send + Sync` and cheap to clone or share via `Arc`.
- Does not panic — always returns `Result<Score, ScorerError>`.
- If network-bound: respects `timeout`, exposes retries, uses anyllm for LLM calls.
- Test coverage: positive case, negative case, missing-reference case, malformed-input case.
