# Phase 0 — Tooling Inventory & Constraint Mapping

## 1. Tool inventory

| Name | Category | Surface | Capability summary |
|------|----------|---------|---------------------|
| `evalkit::Run<I,O,R>` (run.rs) | runner | library | Sample-parallel batch runner; per-sample buffered stream; `concurrency`, `trials`, `sample_timeout`, `seed`. |
| `evalkit::Eval<I,R>` (eval.rs) | runner | library facade | Quickstart facade compiling to `Run::builder()`. |
| `evalkit::Task<I,O>` (task.rs) | runner adapter | library | `from_fn`-only closure wrapper implementing `OutputSource`. |
| `evalkit::OutputSource<I,O>` (source.rs) | runner contract | trait | `async produce(&self, &I) -> Result<O, OutputSourceError>` + `produce_with_snapshots` + `metadata()`. |
| `evalkit::Scorer<I,O,R>` (scorer.rs) | scorer contract | trait | `async score(ctx) -> Result<Score>`; `Score::{Binary, Numeric, Structured, Metric, Label}`. |
| `evalkit::ScorerExt` (scorer_ext.rs) | scorer composition | trait | `.and / .or / .not / .then / .map_score / .timeout / .weighted`. |
| `evalkit-runtime` | runner extras | library | Streaming sources, sinks, sharding (`ShardSpec`), regex-PII scrubbers. |
| `evalkit-providers::HttpAcquisition` | OutputSource impl | library | POST a URL with `input_field`, read `output_field`. |
| `evalkit-providers::SubprocessAcquisition` | OutputSource impl | library | Spawn a process; JSON request `{"input": ...}` on stdin, JSON response on stdout; `timeout`. |
| `evalkit-providers::SubprocessScorer` | Scorer impl | library | Subprocess scorer with `PluginHandshake { kind, name, version, schema_version="1" }`. |
| `evalkit-otel::OtelObserver` | passive OutputSource | library | Reads spans from `TraceBackend` keyed on correlation id. |
| `evalkit-scorers-text` | scorer (deterministic) | library | `exact_match`, `contains`, `regex(pat)`, `json_schema(schema)`. |
| `evalkit-scorers-llm::{LlmJudge, CalibratedLlmClassifier}` | scorer (LLM judge) | library | `llm_judge`, `llm_classifier`, `g_eval`, `calibrated_llm_classifier` over `anyllm`; populates `ScorerResources { token_usage, cost_usd }`. |
| `evalkit-scorers-redteam` | scorer (regex risk) | library | `toxicity`, `bias`, `pii_leakage`, `misuse`, `jailbreak_detected`, `policy_adherence`. |
| `evalkit-scorers-embed`, `evalkit-scorers-rag` | scorer (skeletons) | library | Empty stubs. |
| `evalkit-cli` | runner CLI | binary | `evalkit run|diff|watch`; TOML config; JSONL dataset. |
| `evalkit-server` | persistence + review | binary (axum + rusqlite) | Stores `StoredRun`, `AnnotationRecord`, `AlertRule`, `DriftMeasurement`. |
| `evalkit-exporters-langfuse` | observability sink | library | `export_run(result, config)` POSTs traces+scores to Langfuse. |
| `RunResult / SampleResult / TrialResult` (run_result.rs) | result schema | data | `seed: Option<u64>`, `cost_usd: Option<f64>`, per-trial `TokenUsage { input, output, cache_read, cache_write }`. |
| `read_jsonl / write_jsonl` (evalkit) | persistence | library | JSONL round-trip for datasets and results. |

## 2. Capability map (what the suite CAN do today)

- **Parallelism:** sample-level via `Run::builder().concurrency(N)` (default 4 in CLI); trials within a sample run **sequentially** by design (run.rs:127, 181).
- **Isolation:** none. Subprocess providers run in the parent env (filesystem, env vars, network).
- **Scoring primitives:** binary / numeric / labeled / structured / metric scores; arbitrary composition; per-scorer timeout; LLM-as-judge via `anyllm` with cost/token capture in `ScorerResources`.
- **Telemetry:** Langfuse export of run results; OTel as a *passive output source* (not an emitter); SQLite persistence in `evalkit-server`.
- **Cost tracking:** captured per scorer call (`ScorerResources.cost_usd`, `TokenUsage`) and rolled up per `SampleResult`. Captured but not enforced.
- **Reproducibility:** `seed: Option<u64>` field is recorded; `SourceOutput.snapshots` exists. Neither feeds RNG nor enables replay yet.
- **Subprocess plug-in protocol:** `PLUGIN_PROTOCOL_VERSION = "1"`, JSON handshake on stdin/stdout — a stable seam for adapting any external CLI as an OutputSource.
- **Sharding & PII scrubbing:** `ShardedSource<Src>` and `RegexPiiScrubber` live in `evalkit-runtime` (out-of-scope for this suite but available).

## 3. Capability gaps

- **[BLOCKING] No per-trial sandbox.** Coding-agent trials must run inside an isolated, writable copy of a pinned repository. `SubprocessAcquisition` spawns in the parent process env (no chroot, no overlay, no cwd remap). A trial that runs `rm -rf` on the wrong path is a possibility, and cross-trial cache contamination (e.g., agent home dir, MCP indexer state) is the default. Tagged BLOCKING because it both endangers the host and invalidates statistical comparisons (one trial poisons the next).
- **[BLOCKING] No MCP plumbing.** Indexers and editors are described as MCP servers (`[PENDING_INPUT]`), but evalkit has zero MCP code (verified via grep — only a `[ ] McpAcquisition` checkbox in `docs/integrations.md`). The suite must launch and address MCP servers, write the per-agent config files (Claude Code `.mcp.json` / `--mcp-config`, Codex `~/.codex/config.toml [mcp_servers.*]`, OpenCode `opencode mcp add`), and tear them down deterministically.
- **[BLOCKING] No diff-based scorer or test-runner scorer.** Existing scorers operate on `String → String`. Code tasks score by *applying a patch and running tests*. No `Scorer` does that today.
- **[DEGRADED] Trials run sequentially within a sample.** With 4 agents × `K` indexers × `M` editors × ≥3 trials × ~150 tasks, sample-only parallelism is ~3–10× too slow. Trial-level parallelism inside `execute_sample` (run.rs:181) is the missing knob.
- **[DEGRADED] Seed is stored but not consumed.** No deterministic replay, no seeded RNG injection into the agent shim. Mitigatable with N ≥ 3 trials and paired bootstrap, but should be flagged.
- **[DEGRADED] Cost/token tracking is scorer-side only.** Acquisitions don't populate `TokenUsage` or `cost_usd` for the *agent* call. Coding-agent CLIs (`claude --output-format json`, `codex exec` with metadata, etc.) emit usage on stdout that the shim must parse and surface.
- **[NICE-TO-HAVE] No corpus pinning helper.** Repo + commit-hash pinning, dependency lockfile capture, and pre-warmed worktree pools are not abstracted.
- **[NICE-TO-HAVE] No paired-bootstrap statistical primitive.** `evalkit::Comparison` and `compare()` exist but the diff command's exact statistical contract is not the comparison this suite needs (per-task paired bootstrap on combo pairs).

## GATE 0 status

Indexer and editor lists are **not present in context**. Both proceed as `[PENDING_INPUT]`. The design uses named slots `I_k` and `E_m` and never assumes a specific tool set.

# Phase 1 — Targeted Research

## Coding-agent capability matrix (verified)

| Agent | Headless invocation | MCP config surface | Tool restriction | Built-in editor disable | Source |
|-------|---------------------|--------------------|---------------------|--------------------------|--------|
| **Claude Code** | `claude -p "<prompt>" --bare` | `--mcp-config <file-or-json>` (bare mode) | `--allowedTools "Read,Edit,Bash,..."`; `--permission-mode dontAsk\|acceptEdits` | Omit `Edit` from `--allowedTools` and route writes through MCP-provided edit tool | [SOURCE: https://code.claude.com/docs/en/headless] |
| **Codex** | `codex exec` (non-interactive) | `~/.codex/config.toml` → `[mcp_servers.<name>] command/args/env/url/bearer_token_env_var`; per-server `enabled_tools` / `disabled_tools` allow- and deny-lists | `enabled_tools` allowlist per MCP server; `--full-access` skips approvals | Codex's native edit tool cannot be cleanly toggled off via documented CLI flags `[UNCERTAIN]` — fallback to instructing the agent in the prompt to use the MCP-provided editor only | [SOURCE: https://developers.openai.com/codex/mcp]; `--model`, structured-output flags `[ASSUMED]` |
| **OpenCode** | `opencode run "<prompt>"` (or `opencode serve` + `opencode run --attach`) | `opencode mcp add` (config-file format documented but not in fetched excerpt — `[UNCERTAIN]`) | `opencode agent create --permissions bash,read,edit,...` — "anything omitted is denied" | Create an agent profile excluding `edit` and route writes via MCP-provided edit tool | [SOURCE: https://opencode.ai/docs/cli/] |
| **Pi** | `pi -p "<prompt>"` | **No native MCP support.** "No MCP. Build CLI tools with READMEs (see Skills), or build an extension that adds MCP support." | `--tools read,grep,find,ls` allowlist or `--no-tools` | `--no-tools`, then drive everything via prompt | [SOURCE: https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/README.md] |

Implication: Pi cannot consume MCP indexers or MCP editors out of the box. Cells `(Pi, indexer ≠ none, *)` and `(Pi, *, editor ≠ native)` are `[N/A: Pi lacks native MCP]`. Pi is included only as a (Pi, none, native) baseline. This is a structural property of the agent, not a gap in the harness.

## Benchmark prior art

- **SWE-bench Verified** — 500 human-validated Python tasks; saturating: Claude Mythos Preview 93.9% (April 2026), GPT-5.3 Codex 85%, Claude Opus 4.5 80.9% [SOURCE: https://www.swebench.com/verified.html, https://benchlm.ai/coding]. OpenAI has stopped reporting Verified scores due to contamination concerns [SOURCE: https://www.morphllm.com/swe-bench-pro]. Useful as a *known-vocabulary* corpus, not as the discriminator.
- **SWE-bench Pro** — 1,865 tasks across 41 repos; less saturated (best ~57%, average ~25%) [SOURCE: https://labs.scale.com/leaderboard/swe_bench_pro_public]. Stronger discriminator at top of distribution.
- **Multi-SWE-bench (ByteDance)** — 2,132 instances, 8 languages (Python, Java, TS, JS, Go, Rust, C, C++) [SOURCE: https://arxiv.org/abs/2504.02605]. Source for multilingual coverage.
- **SWE-bench Multilingual** — 300 tasks, 9 languages (C, C++, Go, Java, JS/TS, PHP, Ruby, Rust) [SOURCE: https://www.swebench.com/multilingual-leaderboard.html].
- **SWE-PolyBench (AWS)** — 2,110 instances, Java/JS/TS/Python, mix of bug fixes / features / refactors [SOURCE: https://openreview.net/forum?id=n577FC6CKk]. Closest match for refactor-heavy tasks.
- **Aider Polyglot** — 225 Exercism problems, 6 languages; two-attempt eval (model sees test failures, may revise) [SOURCE: https://github.com/Aider-AI/polyglot-benchmark]. Tests *edit-format compliance* explicitly — directly relevant when comparing editor MCPs that enforce different patch formats.

## LLM-as-judge methodology (current consensus)

- Narrow ordinal scales (3–5 levels) with explicit behavioral anchors beat 10-point Likerts; LLM judges exhibit central-tendency bias on broad scales [SOURCE: https://medium.com/@adnanmasood/rubric-based-evals-llm-as-a-judge-methodologies-and-empirical-validation-in-domain-context-71936b989e80].
- **Both-orderings pairwise**: run (A,B) and (B,A); only count agreed verdicts. Untreated pipelines exhibit ~40% inconsistency from position bias alone [SOURCE: https://aclanthology.org/2025.ijcnlp-long.18.pdf].
- Hybrid scoring is now standard: unit tests answer "did it work"; LLM rubric answers "is it readable / efficient / minimal" [SOURCE: https://www.kinde.com/learn/ai-for-software-engineering/best-practice/llm-as-a-judge-done-right-calibrating-guarding-debiasing-your-evaluators/].
- Calibration: re-grade 5–10% of judge verdicts with humans; track agreement drift [SOURCE: https://arize.com/llm-as-a-judge/].
- Four biases to mitigate explicitly: position, verbosity, self-preference, authority [SOURCE: https://www.adaline.ai/blog/llm-as-a-judge-reliability-bias].

## GATE 1 status

- Claude Code, OpenCode, Pi: tool model verified from current docs.
- Codex: MCP config format verified; non-interactive flag set partly verified — `--model` and built-in-edit-disable behavior are `[ASSUMED]` / `[UNCERTAIN]`. Codex matrix rows are runnable; the disable-native-editor mechanism is flagged for shim-time validation.

# Phase 2 — Vocabulary Lock & Metrics Taxonomy

## Definitions

- **agent** — one of the four named coding-agent CLIs (Claude Code, Codex, OpenCode, Pi).
- **indexer** — an MCP server providing code-retrieval/navigation tools, used by an agent during a trial. `none` is a valid indexer slot meaning "no MCP indexer attached".
- **editor** — an MCP server providing code-edit/patch tools. `native` is a valid editor slot meaning "use the agent's built-in editor".
- **combo** — a triple `(agent, indexer, editor)`. The unit being compared.
- **task** — a deterministic problem statement: `(repo @ commit, prompt, success criteria, hidden test set)`.
- **episode** — one execution of one combo against one task: prompt → agent loop → final repo diff. Indivisible unit of agent work.
- **trial** — one episode with a specific seed and trial index. `N` trials per `(combo, task)` pair.
- **seed** — an integer mixed into the agent's sampling temperature controls (where supported) and into trial-id hashing; recorded even when not effective.
- **harness** — the `evalkit` workspace plus the suite-specific bin/runner that wraps each combo as an `OutputSource<TaskInput, TaskOutput>` and the per-task `Scorer`s.
- **judge** — an `evalkit-scorers-llm::LlmJudge` instance applied to a task with a locked rubric prompt and a pinned `(provider, model)`.
- **verdict** — the final scalar produced by aggregating all scorers for one trial; gates "trial passed".

## Metric set

Computation form: `metric_name :: TrialRecord → MetricValue` (or `Vec<TrialRecord> → MetricValue` for sample-level metrics). Artifact column names the *primary* source field.

| # | Name | Group | Type | Computation | Artifact |
|---|------|-------|------|-------------|----------|
| 1 | `pass_at_1` | correctness | bool | First-trial-only: hidden test suite exits 0 after applying agent diff. | `TrialRecord.test_runner.exit_code` (trial 0) |
| 2 | `pass_rate_at_n` | correctness | f64 in [0,1] | `count(trials where tests pass) / N` over all `N` trials of `(combo, task)`. | `TrialRecord.test_runner.exit_code` |
| 3 | `test_pass_fraction` | correctness | f64 in [0,1] | `passed_count / total_count` of unit tests in hidden suite (graded). | `TrialRecord.test_runner.passed_count`, `total_count` |
| 4 | `regression_rate` | correctness | f64 in [0,1] | `count(trials where pre-existing tests fail post-diff) / N`. | `TrialRecord.test_runner.regressed_count` |
| 5 | `diff_size_loc` | edit_quality | u64 | `lines_added + lines_removed` from the agent's final diff vs starting commit. | `TrialRecord.diff_stats.added + .removed` |
| 6 | `blast_radius` | edit_quality | u64 | Count of files modified outside the task's `expected_scope` glob set. | `TrialRecord.diff_stats.files_changed`, `Task.expected_scope` |
| 7 | `unintended_changes` | edit_quality | u64 | Hunks in the diff that touch lines unrelated to the task (judged binary per-hunk by an LLM judge with rubric `irrelevant_hunk_v1`, summed). | `TrialRecord.judge.unintended_hunks` |
| 8 | `wall_time_s` | efficiency | f64 | Monotonic clock span from `produce()` enter to exit. | `TrialRecord.timing.wall_time_s` |
| 9 | `agent_turns` | efficiency | u32 | Count of agent assistant messages in the agent's session JSONL / stream-json log. | `TrialRecord.agent_telemetry.turns` |
| 10 | `tokens_in` | efficiency | u64 | Sum of input tokens across all agent turns (and judge passes counted separately). | `TrialRecord.agent_telemetry.tokens_in` |
| 11 | `tokens_out` | efficiency | u64 | Sum of output tokens across all agent turns. | `TrialRecord.agent_telemetry.tokens_out` |
| 12 | `indexer_calls` | efficiency | u32 | Count of MCP tool invocations whose server name is the indexer slot. | `TrialRecord.mcp_telemetry.indexer_call_count` |
| 13 | `editor_calls` | efficiency | u32 | Count of MCP tool invocations whose server name is the editor slot (or count of native-edit-tool calls when editor=native). | `TrialRecord.mcp_telemetry.editor_call_count` |
| 14 | `cost_usd` | cost | f64 | `tokens_in * input_rate + tokens_out * output_rate` using the `(provider, model)` pricing table; merged with `cost_usd` already reported by the agent JSON when present. | `TrialRecord.cost.total_usd` |
| 15 | `cost_per_pass` | cost | f64 | `sum(cost_usd) / max(1, count(trials passed))` over all trials of `(combo, task)`. | derived from #14 + #2 |
| 16 | `plan_coherence` | behavioral | u8 in {1..5} | LLM judge rubric `plan_coherence_v1` over agent turn-1 plan plus first 3 turns of execution; both-orderings pairwise vs anchor reference. | `TrialRecord.judge.plan_coherence` |
| 17 | `recovery_rate` | behavioral | f64 in [0,1] | `count(failed_tool_calls_followed_by_corrected_call) / count(failed_tool_calls)`; failed = MCP/tool error response. | `TrialRecord.agent_telemetry.tool_calls[]` |
| 18 | `stopping_quality` | behavioral | u8 in {1..3} | LLM judge rubric `stopping_v1`: 1 = stopped early (tests still red), 2 = stopped on green, 3 = continued past green (over-edit). | `TrialRecord.judge.stopping` |

## GATE 2 status

18 metrics defined, each with a unique computation rule and a single primary artifact field. Every metric in the table is committed to the suite — no "could measure" hedges.

# Phase 3 — Test Matrix & Isolation Design

## 1. Combination space

Let `K` = number of indexer MCP servers supplied at `[PENDING_INPUT]` resolution; let `M` = number of editor MCP servers supplied at `[PENDING_INPUT]` resolution. Always include the synthetic slots `indexer=none` and `editor=native`, raising the slot counts to `K+1` and `M+1`.

Cartesian product = `4 × (K+1) × (M+1)` combos.

| Agent | Indexer slots allowed | Editor slots allowed | Cell status |
|-------|------------------------|----------------------|-------------|
| Claude Code | all `K+1` | all `M+1` | runnable (verified) |
| Codex | all `K+1` | all `M+1` | runnable; native-editor-disable mechanism `[UNCERTAIN]` — cell `(Codex, *, editor≠native)` is `[DEGRADED]` until shim verifies an MCP-only edit path |
| OpenCode | all `K+1` | all `M+1` | runnable; per-combo `agent profile` with `--permissions` controls native-tool exposure |
| Pi | only `indexer=none` | only `editor=native` | only the cell `(Pi, none, native)` is runnable; all other Pi cells are `[N/A: Pi has no native MCP]` |

Effective runnable cells: `3 × (K+1) × (M+1) + 1`. All other Pi cells emit a `combo_skipped` record for completeness.

## 2. Task corpus design

Categories (must each be a distinct `Task.category` value), tasks per category, and sourcing:

| Category | Description | Tasks | Sourcing |
|----------|-------------|-------|----------|
| `single_file_fix` | Failing test → minimal fix touching one file. | 30 | Curated subset of SWE-bench Verified [SOURCE: https://www.swebench.com/verified.html] filtered to single-file patches. |
| `multi_file_refactor` | Rename / extract / move spanning ≥3 files; tests must still pass. | 25 | Curated subset of SWE-PolyBench refactor instances [SOURCE: https://openreview.net/forum?id=n577FC6CKk]. |
| `repo_wide_rename` | Rename a public symbol; suite of grep-based and compile checks. | 15 | Synthetic, derived from this evalkit repo's recent `Acquisition→OutputSource` rename pattern as a template. |
| `feature_with_tests` | Add a feature given a spec + acceptance tests; tests are hidden until grading. | 25 | Curated from Aider Polyglot [SOURCE: https://github.com/Aider-AI/polyglot-benchmark]; pinned. |
| `debug_from_failing_test` | Stack trace + failing test, no diagnosis hint. | 25 | Multi-SWE-bench bug subset, filtered for ≤300 LoC patches [SOURCE: https://arxiv.org/abs/2504.02605]. |
| `large_context_navigation` | Question whose answer requires reading ≥10 files; output is a written diff plus a one-line answer scored by exact match. | 30 | Synthetic, drawn from SWE-bench Pro's `>50k LoC` slice [SOURCE: https://labs.scale.com/leaderboard/swe_bench_pro_public]. |
| **Total** | | **150** | |

Language coverage across the corpus: Python ≥40%, TypeScript ≥15%, Rust ≥15%, Go ≥10%, Java ≥10%, C++ ≥5%, balanced per category where possible.

## 3. Repository corpus

Size buckets and counts:

| Bucket | LoC | Task count | Notes |
|--------|-----|-----------:|-------|
| Small | < 1,000 | 30 | Synthetic single-purpose repos pinned to a tag. |
| Medium | 1,000 – 50,000 | 90 | Real OSS repos at a pinned `commit_sha`. |
| Large | > 50,000 | 30 | Real OSS repos at a pinned `commit_sha`; `large_context_navigation` lives mostly here. |

Pinning policy: every repo recorded as `(git_url, commit_sha, lockfile_path, dockerfile_or_devcontainer_ref)`. No floating refs. Commit SHAs are hard-coded into `tasks/<id>.toml` and CI fails if a repo's git history rewrites a referenced SHA.

## 4. Trial design

- **N = 3 trials** per `(combo, task)`.
- **Seed strategy:** `seed = blake3(task_id || combo_id || trial_index) mod 2^31`. Recorded even when the agent CLI doesn't accept it.
- **Variance-reduction strategy: paired comparison.** Every combo runs every task; comparisons across combos are paired per task. Independent samples are *not* used. Justification: between-task variance dominates between-combo variance for code tasks (some tasks are harder for everyone), so pairing strips the largest noise component and yields ~3–5× the power of independent-samples designs at fixed N.

## 5. Isolation scheme

- **Per-trial fresh worktree.** Suite runner creates a `git worktree add --detach` of the pinned commit into `/var/eval/work/<trial_id>/repo`. Worktrees are removed on trial exit regardless of outcome.
- **Network egress policy.** Default-deny via Linux `unshare -n` plus an explicit per-task allowlist (e.g., `pypi.org`, `crates.io` for build resolution; agent provider API host). MCP server hosts must be on the allowlist.
- **Indexer state reset.** Each trial gets a per-trial indexer working directory; indexer process is started fresh at trial start and SIGTERM'd at trial exit. Index caches under `/var/eval/work/<trial_id>/indexer-state/` are not shared.
- **Agent home isolation.** Per-trial `HOME=/var/eval/work/<trial_id>/home` so agent CLIs (`~/.claude.json`, `~/.codex/config.toml`, `~/.local/share/opencode`, `~/.pi/`) write into the trial sandbox.
- **Cache hygiene.** Build caches (cargo, pnpm, pip wheels) are mounted **read-only** from a host-shared store; any writes go to a per-trial overlay.
- **Process budget.** Per-trial wall-clock cap = `Task.max_wall_time_s` (default 600s); per-trial agent token cap = `Task.max_tokens` (default 200k); per-trial USD cap = `Task.max_cost_usd` (default $5).

## 6. Statistical plan

- **Test:** stratified paired bootstrap on per-task scores. For each pair of combos `(C_a, C_b)` and each metric of interest (`pass_rate_at_n`, `cost_per_pass`, `wall_time_s`):
  1. For each task `t`, compute `Δ_t = score(C_a, t) − score(C_b, t)` (per-task mean across N trials).
  2. Stratify by `Task.category`.
  3. Resample tasks within strata with replacement, B = 10,000 times, computing the mean Δ each time.
  4. Report `mean(Δ)` with the 95% percentile interval; reject H0 if 0 ∉ CI.
- **Unit of analysis:** the task. `(combo, task)` mean across N trials is the data point. This avoids treating correlated trial repeats as independent observations.
- **Multiple-comparison correction:** Holm–Bonferroni across the set of pre-registered combo pairs (not all `C(combos, 2)`). Pre-registered pairs are declared in the report config.
- **Minimum detectable effect (paired):** with 150 tasks, B = 10,000, α = 0.05, power ≥ 0.8, paired bootstrap on `pass_rate_at_n` detects an absolute mean difference of **≥ 5 pp** when per-task pass rates are between 30% and 70% [ASSUMED — verify empirically with a pilot run on 30 tasks before locking corpus size].
- **Required sample size:** 150 tasks × 3 trials × `(3·(K+1)·(M+1) + 1)` combos. With `K = M = 3`, that's `150 × 3 × 49 = 22,050` trials.
- **Pre-registration:** every comparison the report will make is committed to `reports/preregistration.yaml` before the suite runs. Post-hoc comparisons are reported separately and labeled exploratory.

## GATE 3 status

Every cell is either runnable or carries an `[N/A: reason]` / `[DEGRADED: reason]` tag. Pi-with-MCP cells are explicitly N/A. Codex-with-non-native-editor is DEGRADED pending shim-time verification. No silent omissions.

# Phase 4 — Test Suite Specification

## Suite Layout

```
evalkit-codeagents/                       # new top-level crate, member of workspace
├── Cargo.toml                            # bin = evalkit-codeagents, depends on evalkit, evalkit-providers, evalkit-scorers-{text,llm}, evalkit-runtime
├── src/
│   ├── main.rs                           # CLI: `run`, `report`, `pin-repos`
│   ├── combo.rs                          # ComboSpec, combo_to_output_source()
│   ├── task.rs                           # TaskSpec loader, JSONL emitter for evalkit::Dataset
│   ├── shim.rs                           # spawning per-agent shim binaries; capturing AgentTelemetry
│   ├── isolation.rs                      # worktree + unshare(-n) + per-trial HOME
│   ├── mcp.rs                            # launching MCP indexer/editor servers; emitting agent-specific config files
│   ├── scoring.rs                        # TestRunnerScorer, DiffStatsScorer, BlastRadiusScorer, JudgeScorers, telemetry-derived scorers
│   ├── stats.rs                          # paired_bootstrap(), holm_bonferroni()
│   └── report.rs                         # markdown + JSON renderer
├── bin/                                  # subprocess adapter shims (one per agent), each obey PluginHandshake
│   ├── shim-claude-code/                 # Rust crate, releases `evalkit-shim-claude-code`
│   ├── shim-codex/
│   ├── shim-opencode/
│   └── shim-pi/
├── tasks/
│   └── <task_id>.toml                    # one TaskSpec per task (150 files)
├── combos/
│   └── <combo_id>.toml                   # one ComboSpec per combo (4 × (K+1) × (M+1) files, generated)
├── corpus/
│   └── <repo_id>/                        # pinned-commit reference repo (or just a manifest with git_url + sha)
│       └── repo.toml
├── judges/
│   ├── plan_coherence_v1.md              # rubric prompt (3-level Likert with anchors)
│   ├── stopping_v1.md
│   └── irrelevant_hunk_v1.md
└── reports/
    ├── preregistration.yaml              # which combo pairs to compare, alpha, MDE
    └── runs/<run_id>/                    # output: results.jsonl, summary.json, report.md
```

## Task Schema

```rust
// tasks/<id>.toml deserializes to:
struct TaskSpec {
    id: String,                          // unique, kebab-case, stable
    category: TaskCategory,              // single_file_fix | multi_file_refactor | repo_wide_rename | feature_with_tests | debug_from_failing_test | large_context_navigation
    repo_id: String,                     // FK to corpus/<repo_id>/repo.toml
    repo_commit_sha: String,             // 40-char hex; pinned even though repo.toml has it (defense against accidental drift)
    prompt: String,                      // user-visible instructions
    expected_scope: Vec<String>,         // glob patterns; files outside count toward blast_radius
    hidden_test_cmd: Vec<String>,        // argv to run after applying diff; exit-0 => pass
    pre_existing_test_cmd: Option<Vec<String>>,  // for regression_rate; if None, regression_rate = 0
    setup_cmd: Option<Vec<String>>,      // run once after worktree creation, before agent
    max_wall_time_s: u32,                // default 600
    max_tokens: u64,                     // default 200_000
    max_cost_usd: f64,                   // default 5.0
    language_primary: Language,          // python | typescript | javascript | rust | go | java | cpp | c
    loc_bucket: LocBucket,               // small | medium | large
    seed_inputs: Vec<String>,            // optional starting hints injected into prompt; stable
    judge_inputs: JudgeInputs,           // canonical reference diff, plan rubric anchors
    tags: Vec<String>,
}

struct JudgeInputs {
    plan_anchor: String,                 // 1-5 sentence reference plan (not shown to agent)
    stopping_anchor: String,             // human-described "right time to stop"
    irrelevant_hunk_examples: Vec<String>,
}

enum TaskCategory { /* as above */ }
enum Language { /* as above */ }
enum LocBucket { Small, Medium, Large }
```

## Combo Schema

```rust
// combos/<id>.toml deserializes to:
struct ComboSpec {
    id: String,                          // e.g. "claude-code__none__native"
    agent: AgentKind,
    agent_model: String,                 // e.g. "claude-opus-4-7", "gpt-5.4-codex", "anthropic/claude-opus-4-7"
    agent_extra_flags: Vec<String>,      // appended verbatim to the shim invocation
    indexer: IndexerSpec,                // None means "no indexer attached"
    editor: EditorSpec,                  // Native means "use built-in"
    status: ComboStatus,                 // runnable | degraded | not_applicable
    status_reason: Option<String>,       // populated for degraded / not_applicable
}

enum AgentKind { ClaudeCode, Codex, OpenCode, Pi }

enum ComboStatus { Runnable, Degraded, NotApplicable }

enum IndexerSpec {
    None,
    Mcp { id: String, launch: McpLaunch, server_name_for_agent: String },
}

enum EditorSpec {
    Native,
    Mcp { id: String, launch: McpLaunch, server_name_for_agent: String },
}

enum McpLaunch {
    Stdio { command: String, args: Vec<String>, env: BTreeMap<String, String>, cwd: Option<String> },
    Http  { url: String, bearer_token_env_var: Option<String> },
}
```

## Run Schema

One trial → one `TrialRecord` line in `results.jsonl`. Schema is a strict superset of `evalkit::TrialResult` — the suite-specific fields are nested under `extras`.

```rust
struct TrialRecord {
    // identifiers
    run_id: String,                      // run-level UUID (from RunMetadata)
    sample_id: String,                   // = task.id
    trial_index: u32,                    // 0..N-1
    combo_id: String,                    // FK to ComboSpec.id
    seed: u64,                           // blake3(task_id || combo_id || trial_index) mod 2^31

    // verdict
    passed: bool,                        // = test_runner.exit_code == 0
    score_breakdown: Vec<ScoreOutcome>,  // evalkit::ScoreOutcome — one per scorer

    // execution
    timing: Timing,
    cost: Cost,
    agent_telemetry: AgentTelemetry,
    mcp_telemetry: McpTelemetry,
    diff_stats: DiffStats,
    test_runner: TestRunnerOutcome,
    judge: JudgeOutcomes,
    isolation: IsolationRecord,

    // raw artifacts (paths into reports/runs/<run_id>/artifacts/)
    artifact_paths: ArtifactPaths,
}

struct Timing { wall_time_s: f64, agent_phase_s: f64, scoring_phase_s: f64 }

struct Cost { total_usd: f64, by_actor: BTreeMap<String, f64> /* "agent" | "judge" */ }

struct AgentTelemetry {
    turns: u32,
    tokens_in: u64,
    tokens_out: u64,
    tool_calls: Vec<ToolCallRecord>,     // chronological
    raw_session_path: String,            // path to agent's session JSONL / stream-json
}

struct ToolCallRecord {
    tool_name: String,                   // e.g. "Edit", "indexer.search", "editor.apply_patch"
    server: ToolServer,                  // builtin | indexer | editor
    success: bool,
    latency_ms: u32,
    input_excerpt: String,               // truncated to 1KB
    output_excerpt: String,              // truncated to 1KB
}

enum ToolServer { Builtin, Indexer, Editor }

struct McpTelemetry {
    indexer_call_count: u32,
    editor_call_count: u32,
    indexer_error_count: u32,
    editor_error_count: u32,
}

struct DiffStats {
    files_changed: Vec<String>,
    added: u64,
    removed: u64,
    hunks: u32,
    out_of_scope_files: Vec<String>,     // files_changed minus task.expected_scope match
}

struct TestRunnerOutcome {
    exit_code: i32,
    passed_count: u32,
    total_count: u32,
    regressed_count: u32,                // pre-existing tests that flipped to failing
    stdout_path: String,
    stderr_path: String,
}

struct JudgeOutcomes {
    plan_coherence: u8,                  // 1..5 from rubric plan_coherence_v1
    stopping: u8,                        // 1..3 from rubric stopping_v1
    unintended_hunks: u32,               // count from rubric irrelevant_hunk_v1 over all hunks
    judge_token_cost_usd: f64,
}

struct IsolationRecord {
    worktree_path: String,
    home_path: String,
    network_allowlist: Vec<String>,
    aborted_reason: Option<AbortReason>, // None on success
}

enum AbortReason { WallTimeExceeded, TokenBudgetExceeded, CostBudgetExceeded, AgentExitNonZero, ShimCrashed, NetworkPolicyViolation }

struct ArtifactPaths {
    final_diff: String,                  // unified diff, applied vs starting commit
    agent_log: String,
    indexer_log: Option<String>,
    editor_log: Option<String>,
    test_runner_log: String,
}
```

## Harness Contract

The runner is a thin layer on the existing evalkit interfaces. Required surface:

```rust
// evalkit-codeagents/src/combo.rs
fn combo_to_output_source(spec: &ComboSpec, judge_cfg: &JudgeConfig)
    -> Result<Box<dyn OutputSource<TaskInput, TaskOutput>>, ComboError>;
//   For agent ∈ {ClaudeCode, Codex, OpenCode, Pi}, returns a SubprocessAcquisition
//   wrapping bin/shim-<agent>. The shim receives ComboSpec as JSON on stdin via the
//   evalkit-providers PluginHandshake (PLUGIN_PROTOCOL_VERSION = "1") plus the TaskInput,
//   and emits a TaskOutput JSON. Implements OutputSource via SubprocessAcquisition.

struct TaskInput { task_id: String, repo_path: String, prompt: String, mcp_indexer_config_path: Option<String>, mcp_editor_config_path: Option<String>, agent_extra_flags: Vec<String>, agent_model: String, seed: u64, max_wall_time_s: u32, max_tokens: u64, max_cost_usd: f64 }

struct TaskOutput { final_diff_path: String, agent_session_path: String, agent_telemetry: AgentTelemetry, mcp_telemetry: McpTelemetry, isolation: IsolationRecord }
```

```rust
// evalkit-codeagents/src/isolation.rs
fn prepare_trial_workspace(repo: &RepoSpec, trial_id: &str)
    -> Result<TrialWorkspace, IsolationError>;
//   git worktree add --detach @ commit_sha; sets HOME, mounts caches RO,
//   wraps subsequent process spawns with `unshare -n` + per-task allowlist.

fn teardown_trial_workspace(ws: TrialWorkspace) -> Result<(), IsolationError>;
//   SIGTERM all children, remove worktree, persist logs into reports/runs/<run_id>/artifacts/<trial_id>/.
```

```rust
// evalkit-codeagents/src/mcp.rs
fn launch_mcp_servers(combo: &ComboSpec, ws: &TrialWorkspace)
    -> Result<McpHandles, McpError>;
//   For each Mcp slot, spawn the configured Stdio/Http server in the trial network namespace,
//   write per-agent MCP config files into ws.home_path:
//     ClaudeCode -> ws.home/.claude/mcp.json (passed as --mcp-config)
//     Codex      -> ws.home/.codex/config.toml [mcp_servers.<name>]
//     OpenCode   -> ws.home/.local/share/opencode/mcp.json (or equivalent — see [UNCERTAIN] note)
//     Pi         -> error: ComboStatus must be NotApplicable (Pi has no native MCP)

fn shutdown_mcp_servers(handles: McpHandles) -> Result<(), McpError>;
```

```rust
// evalkit-codeagents/src/main.rs
fn run(args: RunArgs) -> Result<RunReport, RunError>;
//   1. Load TaskSpecs from tasks/ filtered by --filter.
//   2. Load ComboSpecs from combos/ filtered by --combo.
//   3. For each combo, build OutputSource via combo_to_output_source.
//   4. Build ScorerSet (see Scoring Contract).
//   5. Construct Run::builder() with concurrency from --concurrency, trials = --trials (default 3),
//      sample_timeout = max_wall_time_s + scoring_overhead.
//   6. Drive evalkit::Run::execute() and write TrialRecord lines to results.jsonl.
//   7. Persist via evalkit-server (optional --server-url).
```

```rust
// evalkit-codeagents/src/stats.rs
fn paired_bootstrap<M: Metric>(records: &[TrialRecord], a: &str, b: &str, metric: M, b_iter: u32, alpha: f64) -> BootstrapResult;
fn holm_bonferroni(p_values: &[f64]) -> Vec<f64>;
```

The harness MUST NOT introduce its own runner — `evalkit::Run` and `evalkit::Eval` are the executors.

## Scoring Contract

Every Phase-2 metric maps to one scorer or one Run-record-derived computation. Scorers consume `TaskOutput` and the trial workspace; metric extraction is a pure function over `TrialRecord`.

```rust
// evalkit-codeagents/src/scoring.rs

// Wraps task.hidden_test_cmd / task.pre_existing_test_cmd; populates TestRunnerOutcome.
struct TestRunnerScorer { task: TaskSpec }
impl Scorer<TaskInput, TaskOutput, TaskRef> for TestRunnerScorer { /* runs cmds; emits Score::Binary + Score::Numeric (test_pass_fraction) + populates ScorerResources */ }

// Computes diff_size_loc + blast_radius from TaskOutput.final_diff_path + task.expected_scope.
struct DiffStatsScorer { task: TaskSpec }
impl Scorer<TaskInput, TaskOutput, TaskRef> for DiffStatsScorer { /* git apply --check then numstat; emits Score::Structured */ }

// Evaluates unintended_hunks via LlmJudge with rubric judges/irrelevant_hunk_v1.md.
struct IrrelevantHunkScorer { judge: LlmJudge }
impl Scorer<TaskInput, TaskOutput, TaskRef> for IrrelevantHunkScorer { /* per-hunk classify */ }

// Plan + stopping rubrics; both-orderings pairwise wrapping internally.
struct PlanCoherenceScorer { judge: LlmJudge, anchor: String }
struct StoppingScorer { judge: LlmJudge, anchor: String }

// Telemetry-derived; no LLM calls. Pulls from TaskOutput.agent_telemetry / mcp_telemetry.
struct AgentTelemetryScorer;       // emits Score::Metric for tokens_in/out, agent_turns
struct McpTelemetryScorer;         // emits Score::Metric for indexer_calls, editor_calls
struct RecoveryRateScorer;         // emits Score::Numeric

// Cost rolled up from agent + judge ScorerResources.
struct CostRollupScorer { pricing: PricingTable }
impl Scorer<TaskInput, TaskOutput, TaskRef> for CostRollupScorer { /* tokens × rate + agent-reported cost; emits Score::Metric */ }

// Composition (built by evalkit::ScorerSet builder):
//   TestRunnerScorer
//     .then(DiffStatsScorer)
//     .then(IrrelevantHunkScorer)        // expensive: judges hunks; .timeout(Duration::from_secs(120))
//     .then(PlanCoherenceScorer)         // expensive
//     .then(StoppingScorer)
//     .then(AgentTelemetryScorer)
//     .then(McpTelemetryScorer)
//     .then(RecoveryRateScorer)
//     .then(CostRollupScorer)
```

Metric → field mapping:

| Metric | Source field on `TrialRecord` | Computed by |
|--------|--------------------------------|-------------|
| 1 `pass_at_1` | `test_runner.exit_code` (filter `trial_index == 0`) | derived in report.rs |
| 2 `pass_rate_at_n` | `test_runner.exit_code` aggregated | derived in report.rs |
| 3 `test_pass_fraction` | `test_runner.passed_count / .total_count` | `TestRunnerScorer` |
| 4 `regression_rate` | `test_runner.regressed_count` aggregated | `TestRunnerScorer` |
| 5 `diff_size_loc` | `diff_stats.added + .removed` | `DiffStatsScorer` |
| 6 `blast_radius` | `diff_stats.out_of_scope_files.len()` | `DiffStatsScorer` |
| 7 `unintended_changes` | `judge.unintended_hunks` | `IrrelevantHunkScorer` |
| 8 `wall_time_s` | `timing.wall_time_s` | shim + harness |
| 9 `agent_turns` | `agent_telemetry.turns` | `AgentTelemetryScorer` |
| 10 `tokens_in` | `agent_telemetry.tokens_in` | `AgentTelemetryScorer` |
| 11 `tokens_out` | `agent_telemetry.tokens_out` | `AgentTelemetryScorer` |
| 12 `indexer_calls` | `mcp_telemetry.indexer_call_count` | `McpTelemetryScorer` |
| 13 `editor_calls` | `mcp_telemetry.editor_call_count` | `McpTelemetryScorer` |
| 14 `cost_usd` | `cost.total_usd` | `CostRollupScorer` |
| 15 `cost_per_pass` | derived from #14 + #2 | report.rs |
| 16 `plan_coherence` | `judge.plan_coherence` | `PlanCoherenceScorer` |
| 17 `recovery_rate` | (computed from `agent_telemetry.tool_calls`) | `RecoveryRateScorer` |
| 18 `stopping_quality` | `judge.stopping` | `StoppingScorer` |

Every Phase-2 metric appears in this table. No orphan metrics.

## Reporting

`report.rs` emits one markdown report and one JSON artifact per run:

```
reports/runs/<run_id>/
├── report.md                    # human-readable
├── summary.json                 # machine-readable
└── artifacts/<trial_id>/        # logs, diffs
```

Required tables in `report.md`:

1. **Pass rate matrix** — rows = combos, columns = task categories, cells = `pass_rate_at_n` ± 95% bootstrap CI. Cells `[N/A]` are dashes.
2. **Cost-efficiency frontier** — scatter of `(cost_per_pass, pass_rate_at_n)` per combo; Pareto front highlighted.
3. **Edit-quality table** — rows = combos, columns = `diff_size_loc` (median), `blast_radius` (mean), `unintended_changes` (mean).
4. **MCP tool intensity** — rows = combos, columns = `indexer_calls`, `editor_calls`, `recovery_rate`.
5. **Behavioral judge table** — rows = combos, columns = `plan_coherence` (mean), `stopping_quality` distribution.
6. **Pairwise combo comparison** — for each pre-registered pair, paired-bootstrap result on the agreed-on primary metric.

**Winner-determination gate.** A combo `C_a` is declared the winner over `C_b` *only* if all of the following hold for the run, evaluated against `reports/preregistration.yaml`:
1. `pass_rate_at_n(C_a) − pass_rate_at_n(C_b) ≥ 0.05` (5 pp absolute) with 95% paired-bootstrap CI excluding 0 (Holm–Bonferroni corrected within the pre-registered comparison set).
2. `regression_rate(C_a) ≤ regression_rate(C_b) + 0.02`.
3. `cost_per_pass(C_a) ≤ 1.5 × cost_per_pass(C_b)` (cost regression ceiling).
4. `blast_radius(C_a)` does not exceed `blast_radius(C_b)` by more than 1.5 files on average.

If conditions 1–4 are not jointly satisfied, the report records `no_winner` for that pair.

## Reproducibility checklist

```
- pinned_versions:
    - evalkit-codeagents:        commit_sha + Cargo.lock hash
    - evalkit (workspace):       commit_sha
    - rustc:                     pinned via rust-toolchain.toml
    - each agent CLI:            { kind, version_string captured by `<agent> --version` at trial start, sha256 of binary }
    - each MCP server image:     digest pin (oci@sha256:...)
- environment_capture (per trial):
    - HOME, PATH, env-allowlist
    - kernel version (uname -a)
    - libc version
    - network namespace label
    - cache mount snapshot id
- seed_recording:
    - per-trial seed in TrialRecord.seed
    - run-level seed in RunMetadata.seed (used to seed task ordering RNG)
- task corpus pin:
    - tasks/ tree commit_sha (this suite repo)
    - corpus/ repo manifests with commit_shas; hashes verified at trial start
- judge pin:
    - judges/<rubric>.md sha256 stored on each TrialRecord that used it
    - judge (provider, model) string pinned in JudgeConfig
- pricing pin:
    - PricingTable version stamp on each TrialRecord (used for cost backfill)
- preregistration:
    - reports/preregistration.yaml committed before run; sha256 in RunMetadata
```

## GATE 4 status

Every Phase-2 metric (#1–#18) maps to a `TrialRecord` field and a Scoring-Contract function (or a derivation explicitly named in `report.rs`). No metric is dangling.

# Phase 5 — Tooling Recommendations

Five gaps were tagged `[BLOCKING]` or `[DEGRADED]` in Phase 0. One recommendation each.

## [BLOCKING] No per-trial sandbox

**Recommendation:** add a `evalkit-codeagents/src/isolation.rs` module that uses `git worktree add --detach` for filesystem isolation, `unshare -Urn` for a per-trial network namespace seeded from a per-task host allowlist (resolved once and inserted into a userspace nftables ruleset), and a per-trial `HOME` under the worktree. No Docker required for the common case; reserve Docker (`docker run --rm --network=eval_<trial>`) only for tasks with `setup_cmd` that cannot run unprivileged. Build cost: ~2 engineer-weeks. Ongoing cost: linux-only host (acceptable for an internal eval suite).

## [BLOCKING] No MCP plumbing

**Recommendation:** implement `evalkit-codeagents/src/mcp.rs` as a thin launcher with **per-agent config emitters** rather than a generic MCP client. The harness does not connect to MCP servers itself — the agent CLI does. The harness's job is (a) spawn the MCP server process(es) in the trial namespace and (b) write the agent's MCP config file into the trial `HOME`. Per-agent emitters: Claude Code → `~/.claude/mcp.json` referenced via `--mcp-config` ([SOURCE: https://code.claude.com/docs/en/headless]); Codex → `~/.codex/config.toml` `[mcp_servers.<name>]` ([SOURCE: https://developers.openai.com/codex/mcp]); OpenCode → equivalent file (`opencode mcp add`'s output, exact path `[UNCERTAIN]` — verify at shim-build time); Pi → unsupported, combo marked `NotApplicable`. Build cost: ~1 engineer-week.

## [BLOCKING] No diff/test scorers

**Recommendation:** add `TestRunnerScorer` and `DiffStatsScorer` to `evalkit-scorers-text` (deterministic) and `IrrelevantHunkScorer`/`PlanCoherenceScorer`/`StoppingScorer` to `evalkit-scorers-llm` (judges). The deterministic scorers run `git apply` and the task's `hidden_test_cmd` inside the worktree; the LLM judges build on the existing `LlmJudge` and `g_eval_with_steps` factories, supplying the rubrics from `judges/*.md`. Build cost: ~1 engineer-week deterministic + ~1 engineer-week judges (including both-orderings pairwise wrapper and a rubric-calibration pass against ~50 human-graded trials).

## [DEGRADED] Sequential trials within a sample

**Recommendation:** parameterize trial concurrency in `evalkit::Run::builder().trial_concurrency(N)` (currently sequential at run.rs:181) with a default of 1 to preserve existing behavior, and call sites set it to `min(8, available_cores / per_trial_cores)` for this suite. The change is local to `execute_sample` and gated on a per-Run knob — no breaking change to the kernel trait. Build cost: ~3 engineer-days.

## [DEGRADED] Cost/token tracking missing on the OutputSource side

**Recommendation:** add `SourceOutput::resources: Option<ScorerResources>` (mirroring the scorer-side type) and have each agent shim emit `tokens_in`, `tokens_out`, `cost_usd`, and `agent_turns` in its JSON response. The harness merges these into the `TrialRecord.cost.by_actor["agent"]` slot. Existing scorer-side cost stays in `by_actor["judge"]`. Build cost: ~3 engineer-days for the kernel field + ~1 day per agent shim.

Sources:
- [Run Claude Code programmatically — Claude Code Docs](https://code.claude.com/docs/en/headless)
- [CLI – Codex | OpenAI Developers](https://developers.openai.com/codex/cli)
- [Model Context Protocol – Codex | OpenAI Developers](https://developers.openai.com/codex/mcp)
- [CLI | OpenCode](https://opencode.ai/docs/cli/)
- [pi-coding-agent README — badlogic/pi-mono](https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/README.md)
- [SWE-bench Verified leaderboard](https://www.swebench.com/verified.html)
- [SWE-Bench Pro public leaderboard](https://labs.scale.com/leaderboard/swe_bench_pro_public)
- [SWE-bench Multilingual leaderboard](https://www.swebench.com/multilingual-leaderboard.html)
- [Multi-SWE-bench paper](https://arxiv.org/abs/2504.02605)
- [SWE-PolyBench OpenReview](https://openreview.net/forum?id=n577FC6CKk)
- [Aider polyglot benchmark](https://github.com/Aider-AI/polyglot-benchmark)
- [Rubric-Based Evaluations & LLM-as-a-Judge — Adnan Masood, April 2026](https://medium.com/@adnanmasood/rubric-based-evals-llm-as-a-judge-methodologies-and-empirical-validation-in-domain-context-71936b989e80)
- [A Systematic Study of Position Bias in LLM-as-a-Judge](https://aclanthology.org/2025.ijcnlp-long.18.pdf)
- [LLM-as-a-Judge: Reliability, Bias — Adaline](https://www.adaline.ai/blog/llm-as-a-judge-reliability-bias)
- [Kinde — LLM-as-a-Judge, Done Right](https://www.kinde.com/learn/ai-for-software-engineering/best-practice/llm-as-a-judge-done-right-calibrating-guarding-debiasing-your-evaluators/)
- [Arize — LLM as a Judge primer](https://arize.com/llm-as-a-judge/)
- [Serena — semantic MCP toolkit](https://github.com/oraios/serena)
- [BenchLM March 2026 coding leaderboard](https://benchlm.ai/coding)
- [morphllm — SWE-Bench Pro analysis](https://www.morphllm.com/swe-bench-pro)
