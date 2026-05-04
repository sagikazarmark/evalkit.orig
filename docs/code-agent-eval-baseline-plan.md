# Code-agent eval suite — baseline plan (V2)

Status: locked baseline, config-first pivot. Iterations should be recorded as deltas against this document.

V1 history: an earlier code-heavy draft lives in `docs/code-agent-eval-suite-spec.md`. V2 reframes the suite as a TOML config bundle plus a set of evalkit kernel/CLI deltas — instead of a custom `evalkit-codeagents` Rust crate. The user-facing artifact for the 80% case is one TOML file.

## Locked decisions

```
Cadence: re-runnable / iterative (operator may run with minor changes between executions)
Budget: time=tier B (target ~4h, accepts ~6.8h at concurrency=12), spend=tier β ($300–$800 per full run), hardware=H1 single Linux host (16–32 cores, 64+ GB RAM, kernel ≥ 5.10)
Decision-supported: multiple — Pareto frontier (cost-per-pass × pass_rate_at_n) as headline; drill-downs = best combo overall, best indexer per agent, best editor per agent, delta-vs-previous-run; visualization required (1 scatter + 4 ranking tables + 1 delta table when prior summary.json present)
Indexers: [I0 none(baseline); I1 codemogger; I2 codedb; I3 axon; I4 Serena; I5 Octocode; I6 Probe; I7 CocoIndex Code; I8 AiDex; I9 Codanna; I10 Srclight; I11 Pathfinder] — launch metadata [PENDING_INPUT] for each MCP
Editors: [E0 native(baseline); E1 Morph Fast Apply; E2 Relace; E3 Mercury Coder Apply-Edit] — launch metadata [PENDING_INPUT] for each MCP
Models: { ClaudeCode: claude-sonnet-4-6, Codex: gpt-5.3-codex [ASSUMED naming], OpenCode: anthropic/claude-sonnet-4-6, Pi: anthropic/claude-sonnet-4-6 }
Concurrency: 12 trials in parallel
Prompt template: locked single template across all combos
Excluded combos: [Pi × indexer != I0, Pi × editor != E0]  (structural [N/A: Pi has no native MCP] only)
Combo count: 145 default-enabled = (3 agents × 12 indexer slots × 4 editor slots) + 1 Pi baseline; every row carries enabled:bool toggleable at run-time
Corpus: { source_mix=60% curated (SWE-PolyBench refactor + Multi-SWE-bench bug subset + Aider polyglot fixed-set) / 20% synthetic / 20% real internal (drop if none), categories=[single_file_fix, multi_file_refactor, debug_from_failing_test, feature_with_tests], languages=[Python>=8, TypeScript>=6, Rust>=6, Go>=4], size_mix=S/M/L=8/20/2, total_tasks=30, pass_criterion=hidden test exit 0 (regression_rate captured non-gating) }
Run model: two-round screening → focused (each round = one `evalkit run` invocation).
  Round 1 (screening): 145 combos × 5 tasks × 1 trial = 725 trials, ~3.0 h wall @ concurrency 12, ~$363; output = ranked list, top 10 promoted.
  Round 2 (focused): 10 combos × 30 tasks × 3 trials = 900 trials, ~3.75 h wall @ concurrency 12, ~$450; output = full report.
Metrics: 19 — correctness (4), edit_quality (2), efficiency (9), cost (2), behavioral (2). Judge cap = $0.05 per task, round 2 only.
Trials per (combo,task): round 1 = 1, round 2 = 3
Variance reduction: paired
Comparison set: pre-registered combo pairs in reports/preregistration.yaml; Holm–Bonferroni within the pre-registered set
Winner gate: { pass_rate_delta = +0.05 (5 pp absolute, 95% paired-bootstrap CI exclude 0), regression_ceiling = +0.02, cost_ceiling = 1.5×, blast_radius_ceiling = +1.5 files mean }
Sandbox: evalkit's [isolation] mode = "worktree" (git worktree + unshare -n); Docker per-trial as escape hatch only for tasks whose setup_cmd cannot run unprivileged
Reproducibility: full pin
Reporting: [report.md, summary.json, per-trial artifacts] on disk; optional evalkit-server push behind --server-url flag
Deliverable: design-spec markdown + evalkit roadmap deltas (no special-purpose Rust crate; all required code lands inside evalkit itself)
```

# 1. Suite layout

```
suite/                                        # the entire user-side artifact
├── suite.toml                                # the 80%: matrix, isolation, scorers, stats, report
├── tasks.jsonl                               # 30 task definitions (or inline [[task]] in suite.toml)
├── corpus/<repo_id>/repo.toml                # pinned-commit repo manifests; referenced from tasks
├── judges/
│   └── stopping_v1.md                        # rubric for type="llm_judge"
├── reports/
│   ├── preregistration.yaml                  # combo pairs to compare; gate thresholds; alpha
│   └── runs/<run_id>/                        # output (created by `evalkit run`)
│       ├── results.jsonl                     # one TrialRecord per line
│       ├── summary.json                      # rolled up
│       ├── report.md
│       └── artifacts/<trial_id>/             # final_diff.patch, agent_session.jsonl, indexer/editor/test_runner logs
└── plugins/                                  # the 20% — only if you need a custom scorer
    └── my_custom_scorer.py                   # invoked via [[scorer]] type = "plugin"
```

No Rust crate. No `bin/shim-*` directories. The four supported coding-agent CLIs are reached via the new built-in `code_agent` acquisition type (see §5). Custom logic uses the existing subprocess plugin protocol (`evalkit-providers::PluginHandshake v1`) — write Python, Bash, anything.

# 2. Task schema (TOML)

A task is a TOML table (or one JSONL line). Fields:

```toml
[[task]]
id = "fix-auth-py"                            # unique, kebab-case, stable
enabled = true                                # run-time toggle
category = "single_file_fix"                  # single_file_fix | multi_file_refactor | debug_from_failing_test | feature_with_tests
language = "python"                           # python | typescript | rust | go
loc_bucket = "small"                          # small | medium | large
prompt = "Fix the bug in auth.py reported by tests/test_auth.py"
expected_scope = ["src/auth.py"]              # globs; files outside count toward blast_radius
hidden_test_cmd = ["pytest", "tests/test_auth.py"]
pre_existing_test_cmd = ["pytest", "--ignore=tests/test_auth.py"]   # optional; for regression_rate
setup_cmd = ["pip", "install", "-e", "."]     # optional; runs once per worktree before agent
max_wall_time_s = 600                         # default 600
max_tokens = 200000                           # default 200_000
max_cost_usd = 5.0                            # default 5.0
tags = ["python", "small"]

[task.repo]
git = "https://github.com/example/repo"
commit = "abc1234567890abcdef1234567890abcdef12345"   # 40-char sha; pinned
lockfile = "requirements.txt"                          # optional, captured for reproducibility

[task.judge_inputs]
stopping_anchor = "Right time to stop = once test_auth.py passes and no other tests regress."
```

JSONL form is the same shape with TOML keys flattened to dotted paths or nested objects.

# 3. Combo schema (TOML matrix expansion)

Combos are not authored individually. They are expanded from `[[matrix.*]]` arrays at run time. Every row carries `enabled` and an `id`; `--include` / `--exclude` accept the auto-generated combo IDs (`{agent_id}__{indexer_id}__{editor_id}`).

```toml
[[matrix.agent]]
id = "claude-code"; enabled = true; cli = "claude"; model = "claude-sonnet-4-6"
mcp_config_target = "claude_json"             # ~/.claude/mcp.json + --mcp-config

[[matrix.agent]]
id = "codex"; enabled = true; cli = "codex"; model = "gpt-5.3-codex"
mcp_config_target = "codex_toml"              # ~/.codex/config.toml [mcp_servers.*]
exec_subcommand = "exec"                      # `codex exec` for non-interactive
extra_flags = ["--full-access"]

[[matrix.agent]]
id = "opencode"; enabled = true; cli = "opencode"; model = "anthropic/claude-sonnet-4-6"
mcp_config_target = "opencode_json"
permissions_profile = ["bash", "read"]        # native edit excluded so MCP editor wins

[[matrix.agent]]
id = "pi"; enabled = true; cli = "pi"; model = "anthropic/claude-sonnet-4-6"
mcp_supported = false                         # auto-NotApplicable for non-baseline indexer/editor cells

[[matrix.indexer]]
id = "serena"; enabled = true
transport = "stdio"; command = "uvx"; args = ["--from", "git+https://github.com/oraios/serena", "serena-mcp-server"]
server_name = "serena"

[[matrix.indexer]]
id = "probe"; enabled = true
transport = "stdio"; command = "probe-mcp"
server_name = "probe"

# … 9 more indexers …

[[matrix.indexer]]
id = "none"; baseline = true                  # synthetic baseline — "no indexer attached"

[[matrix.editor]]
id = "morph"; enabled = true
transport = "http"; url = "https://api.morphllm.com/mcp"; bearer_token_env = "MORPH_API_KEY"
server_name = "morph"

[[matrix.editor]]
id = "relace"; enabled = true
transport = "http"; url = "https://api.relace.ai/mcp"; bearer_token_env = "RELACE_API_KEY"
server_name = "relace"

[[matrix.editor]]
id = "mercury"; enabled = true
transport = "http"; url = "https://api.inceptionlabs.ai/mcp/apply-edit"; bearer_token_env = "INCEPTION_API_KEY"
server_name = "mercury"

[[matrix.editor]]
id = "native"; baseline = true                # synthetic baseline — "use agent's built-in editor"

[[matrix.exclude]]
when = { agent = "pi", indexer = { not = "none" } }
[[matrix.exclude]]
when = { agent = "pi", editor = { not = "native" } }
```

Auto-generated combo status:
- `Pi × non-baseline → NotApplicable` (matrix.exclude rule).
- `Codex × non-native editor → Degraded` until a shim probe verifies a documented native-edit-disable mechanism — until then editor MCP is invoked via prompt-only.
- All other cells → `Runnable`.

Filter at run time: `evalkit run --config suite.toml --include 'claude-code__serena__morph,codex__probe__native'` or `--exclude '*pi*,codex__*'`.

# 4. Run schema / TrialRecord

The on-disk artifact format (one JSONL line per trial in `reports/runs/<run_id>/results.jsonl`). Type signatures stay in Rust because this is the wire format read by `evalkit::Run`, `evalkit-server`, and downstream tooling — not user code.

```rust
struct TrialRecord {
    // identifiers
    run_id: String,                              // RunMetadata UUID
    round: Round,                                // Screen | Focus  (set by --round flag on evalkit run)
    sample_id: String,                           // = task.id
    trial_index: u32,                            // 0..N-1
    combo_id: String,                            // FK to expanded matrix combo id
    seed: u64,                                   // blake3(task_id || combo_id || trial_index) mod 2^31

    // verdict
    passed: bool,                                // = test_runner.exit_code == 0
    score_breakdown: Vec<ScoreOutcome>,          // evalkit::ScoreOutcome — one per scorer

    // execution
    timing: Timing,
    cost: Cost,
    output_telemetry: OutputTelemetry,           // <— K3 kernel addition
    mcp_telemetry: McpTelemetry,
    diff_stats: DiffStats,
    test_runner: TestRunnerOutcome,
    judge: JudgeOutcomes,                        // populated only when round == Focus
    isolation: IsolationRecord,
    artifact_paths: ArtifactPaths,
}

enum Round { Screen, Focus }

struct Timing { wall_time_s: f64, agent_phase_s: f64, scoring_phase_s: f64 }

struct Cost { total_usd: f64, by_actor: BTreeMap<String, f64> /* "agent" | "judge" */ }

struct OutputTelemetry {
    turns: u32,
    tokens_in: u64,
    tokens_out: u64,
    tool_calls: Vec<ToolCallRecord>,             // chronological
    raw_session_path: String,                    // path to agent stream-json / session JSONL
    distinct_files_read: u32,
    distinct_files_written: u32,
    total_file_ops: u32,
}

struct ToolCallRecord {
    seq: u32,
    tool_name: String,                           // e.g. "Read", "indexer.search", "editor.apply_patch"
    server: ToolServer,                          // builtin | indexer | editor
    success: bool,
    latency_ms: u32,
    input_excerpt: String,                       // truncated to 1KB
    output_excerpt: String,                      // truncated to 1KB
    files_touched: Vec<String>,
}

enum ToolServer { Builtin, Indexer, Editor }

struct McpTelemetry { indexer_call_count: u32, editor_call_count: u32, indexer_error_count: u32, editor_error_count: u32 }

struct DiffStats { files_changed: Vec<String>, added: u64, removed: u64, hunks: u32, out_of_scope_files: Vec<String> }

struct TestRunnerOutcome { exit_code: i32, passed_count: u32, total_count: u32, regressed_count: u32, stdout_path: String, stderr_path: String }

struct JudgeOutcomes { stopping: u8, judge_token_cost_usd: f64 }     // stopping = 0 when not run

struct IsolationRecord { worktree_path: String, home_path: String, network_allowlist: Vec<String>, aborted_reason: Option<AbortReason> }

enum AbortReason { WallTimeExceeded, TokenBudgetExceeded, CostBudgetExceeded, AgentExitNonZero, ShimCrashed, NetworkPolicyViolation }

struct ArtifactPaths { final_diff: String, agent_log: String, indexer_log: Option<String>, editor_log: Option<String>, test_runner_log: String }
```

Compatibility note: `OutputTelemetry` is identical to V1's `AgentTelemetry`; renamed because it now lives on `SourceOutput` for any acquisition (kernel addition K3), not just code-agents.

# 5. Suite TOML grammar reference

Replaces V1's "Harness contract." This is the user-visible surface for the 80% case.

## `[run]`

```toml
[run]
isolation         = "worktree"                # worktree | docker | unshare | none | command (escape hatch)
concurrency       = 12                        # sample-level parallelism
trials            = 1                         # per (combo, task); profile-overridable via --trials
sample_timeout_s  = 900                       # default = max_wall_time_s + scoring overhead
seed              = 0xDEADBEEF                # run-level seed for task-ordering RNG
network_allowlist = ["api.anthropic.com", "api.openai.com", "pypi.org", "crates.io"]
```

## `[isolation]`

```toml
[isolation]
mode               = "worktree"               # see [run].isolation
home_root          = "/var/eval/work"         # per-trial HOME goes under here
cache_mounts       = [{ src = "~/.cargo", ro = true }, { src = "~/.cache/pip", ro = true }]
keep_on_failure    = true                     # preserve worktree for debugging when trial fails
```

## `[[matrix.agent]]`, `[[matrix.indexer]]`, `[[matrix.editor]]`, `[[matrix.exclude]]`

See §3.

## `[[task]]` (or `[dataset] file = "tasks.jsonl"`)

See §2.

## `[[scorer]]`

```toml
[[scorer]] type = "test_runner"
[[scorer]] type = "diff_stats"     # reads task.expected_scope automatically
[[scorer]] type = "agent_telemetry"
[[scorer]] type = "mcp_telemetry"
[[scorer]] type = "cost_rollup"

[[scorer]] type = "recovery_rate"; round = "focus"

[[scorer]] type = "llm_judge"
rubric         = "judges/stopping_v1.md"
both_orderings = true                         # mitigate position bias (kernel addition K4)
cap_usd        = 0.05                         # per-task budget; aborts scorer if exceeded
round          = "focus"
output         = { kind = "label", labels = [1, 2, 3] }

# Custom scorer — the 20%
[[scorer]] type = "plugin"
command       = ["python3", "plugins/my_custom_scorer.py"]
timeout_secs  = 60
```

## `[stats]`

```toml
[stats]
comparison    = "paired_bootstrap"            # or "mcnemar" | "wilcoxon"
b_iter        = 10000
alpha         = 0.05
correction    = "holm_bonferroni"             # or "bonferroni" | "none"

[stats.winner_gate]
pass_rate_delta      = 0.05                   # required Δpass_rate_at_n
regression_ceiling   = 0.02                   # max +Δregression_rate
cost_ceiling_ratio   = 1.5                    # cost_per_pass(winner) ≤ 1.5 × loser
blast_radius_ceiling = 1.5                    # files
```

## `[report]`

```toml
[report]
tables = ["pareto", "best_combo", "best_per_agent", "preregistered_pairs", "delta_vs_previous"]
formats = ["markdown", "json"]                # report.md + summary.json
svg     = false                               # render Pareto as inline ASCII unless true

[[report.custom]]
template = "panels/my_panel.tera"             # 20% extension point
```

## CLI invocations

```sh
# Round 1: screening
evalkit run --config suite/suite.toml --trials 1 --task-subset '@first(5)' --round screen

# Inspect the screening result, pick top 10
evalkit report --pareto reports/runs/<screen_id>/results.jsonl --top-by pass_rate_at_n --limit 10

# Round 2: focused
evalkit run --config suite/suite.toml --trials 3 --include $(cat top10.txt | paste -sd,) --round focus

# Final report with delta vs previous
evalkit report --config suite/suite.toml \
  --results reports/runs/<focus_id>/results.jsonl \
  --previous reports/runs/<prior_focus_id>/summary.json
```

# 6. Built-in scorer reference

Every metric in the registry maps to a built-in scorer or a derivation in `evalkit report`. No user code required.

| TOML `type` | Computes | Source artifact | Round |
|-------------|----------|-----------------|-------|
| `test_runner` | `pass_at_1` (derived), `pass_rate_at_n` (derived), `test_pass_fraction`, `regression_rate` | runs `task.hidden_test_cmd` and `task.pre_existing_test_cmd` inside `TrialContext.worktree_path`, parses framework-agnostic output | both |
| `diff_stats` | `diff_size_loc`, `blast_radius` | `git -C <worktree> diff --numstat HEAD` + glob match against `task.expected_scope` | both |
| `agent_telemetry` | `agent_turns`, `tokens_in`, `tokens_out`, `distinct_files_read`, `distinct_files_written`, `total_file_ops` | `OutputTelemetry` populated by `code_agent` acquisition | both |
| `mcp_telemetry` | `indexer_calls`, `editor_calls` | `OutputTelemetry.tool_calls` filtered by `ToolServer` | both |
| `cost_rollup` | `cost_usd`, `cost_per_pass` (derived) | tokens × `[pricing]` table + `OutputTelemetry.cost_usd` if agent reports it | both |
| `recovery_rate` | `recovery_rate` | walks `OutputTelemetry.tool_calls` in seq order; counts (failed → corrected) pairs | focus |
| `llm_judge` (rubric=`stopping_v1.md`) | `stopping_quality` | one LLM call against rubric; both-orderings wrapper applied | focus |

Derivations (computed in `evalkit report`, not by a scorer):
- `pass_at_1` — filter `trial_index == 0`, count `passed`.
- `pass_rate_at_n` — aggregate `passed` over N trials of (combo, task).
- `regression_rate` — aggregate `test_runner.regressed_count > 0` over N trials.
- `cost_per_pass` — `sum(cost_usd) / max(1, count(passed))` per (combo, task).

# 7. Reporting

`reports/runs/<run_id>/`:

```
report.md
summary.json
artifacts/<trial_id>/{ final_diff.patch, agent_session.jsonl, indexer.log, editor.log, test_runner.{stdout,stderr}.log }
```

Required tables in `report.md` (declared in `[report] tables`):

1. **Pareto frontier (scatter)** — `cost_per_pass` × `pass_rate_at_n`, one point per combo, Pareto-optimal combos labeled. Inline ASCII by default; SVG with `[report] svg = true`.
2. **Best combo overall** — combos sorted by `pass_rate_at_n` desc, `cost_per_pass` tiebreaker; CI bands.
3. **Best per agent** — three sub-tables per dimension (one per MCP-capable agent): one ranks indexers, one ranks editors.
4. **Pre-registered pairwise comparisons** — for each pair in `reports/preregistration.yaml`: paired-bootstrap mean Δ, 95% CI, Holm-corrected p, gate verdict.
5. **Delta vs previous run** — populated when `--previous reports/runs/<prev_run_id>/summary.json` supplied; rows = combos in both, columns = Δpass_rate_at_n / Δcost_per_pass / Δblast_radius / regression flag.

Winner-determination gate (declarative, evaluated by `evalkit report` against `[stats.winner_gate]`). A combo `C_a` beats `C_b` iff:
1. `pass_rate_at_n(C_a) − pass_rate_at_n(C_b) ≥ 0.05` with Holm-corrected 95% paired-bootstrap CI excluding 0.
2. `regression_rate(C_a) ≤ regression_rate(C_b) + 0.02`.
3. `cost_per_pass(C_a) ≤ 1.5 × cost_per_pass(C_b)`.
4. `blast_radius_mean(C_a) ≤ blast_radius_mean(C_b) + 1.5`.

Otherwise: `no_winner`.

```rust
struct Summary {
    run_id: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    round: Round,
    config_digest: String,                       // sha256 of merged TOML config
    preregistration_digest: String,
    combos: Vec<ComboSummary>,
    pairwise: Vec<PairwiseResult>,
    delta_vs_previous: Option<DeltaTable>,
}

struct ComboSummary { /* per-combo aggregates: pass_rate_at_n + CI95, cost_per_pass, regression_rate, blast_radius_mean, … */ }
struct PairwiseResult { a: String, b: String, metric: String, mean_delta: f64, ci95: (f64, f64), p_holm: f64, gate_verdict: GateVerdict }
enum GateVerdict { Winner, NoWinner { failed: Vec<GateCondition> } }
enum GateCondition { PassRateDelta, RegressionCeiling, CostCeiling, BlastRadiusCeiling }
```

# 8. Reproducibility checklist

```
- pinned_versions:
    - evalkit, evalkit-cli, evalkit-providers: commit_sha + Cargo.lock hash
    - rustc:                                   pinned via rust-toolchain.toml (host install)
    - each agent CLI:                          { kind, version_string from `<agent> --version`, sha256 of binary }
    - each MCP server:                         OCI digest when containerized; pinned npm/uvx/cargo install spec otherwise
- environment_capture (per trial, recorded in IsolationRecord):
    - HOME, PATH, env-allowlist
    - kernel version (uname -a), libc version
    - network namespace label, cache mount snapshot id
- seed_recording:
    - per-trial seed in TrialRecord.seed
    - run-level seed in [run].seed (task ordering RNG)
- task corpus pin:
    - suite/ tree commit_sha
    - corpus/ repo manifests commit_shas; sha verified at trial start; run aborts on mismatch
- judge pin:
    - judges/<rubric>.md sha256 stored on each TrialRecord that used it
    - judge (provider, model) string pinned in [[scorer]].provider/.model
    - both_orderings flag logged when applied
- pricing pin:
    - [pricing] table version stamp on each TrialRecord
- preregistration:
    - reports/preregistration.yaml committed before run; sha256 in RunMetadata
```

# 9. evalkit roadmap deltas — what evalkit must ship

Twenty-one deltas across kernel, providers/scorers, and CLI. Sorted by severity for the v2 user experience. Once shipped, this suite is one TOML file; every other matrix-shaped, sandboxed, multi-trial eval gets the same primitives for free.

## Kernel (`evalkit`, `evalkit-runtime`)

| # | Delta | Severity | Build |
|---|-------|----------|-------|
| K1 | `IsolationSpec` + `TrialContext { worktree_path, home_path, network_ns, env }` passed to `OutputSource::produce`. Additive: a default `TrialContext::host()` keeps existing impls working. | **BLOCKING** | 1 wk |
| K2 | `WorktreeIsolation`, `UnshareIsolation`, `DockerIsolation` impls of an `IsolationProvider` trait; selected via `[isolation] mode = "..."` in TOML. | **BLOCKING** | 2 wk |
| K3 | `SourceOutput::telemetry: Option<OutputTelemetry { tokens_in, tokens_out, turns, tool_calls, cost_usd, raw_session_path, distinct_files_*, total_file_ops }>`. Acquisitions populate; scorers and reports consume. Generalizes V1's per-shim `AgentTelemetry`. | **BLOCKING** | 3 d |
| K4 | `BothOrderings<S: Scorer>` adapter — runs `(A,B)` then `(B,A)`, returns a verdict only when both agree. Drops position-biased calls, exposed via `[[scorer]].both_orderings = true`. | **BLOCKING** | 2 d |
| K5 | `Run::builder().trial_concurrency(N)`; default 1. Currently sequential at run.rs:181. Suite holds at 1 (paired comparison wants stable per-trial timing) but the knob widens evalkit's design space. | DEGRADED | 3 d |
| K6 | Budget enforcement — `max_cost_usd` / `max_tokens` checked at scorer-call boundary; aborts trial with `AbortReason::CostBudgetExceeded`. Today `ScorerResources.cost_usd` is captured but not enforced. | DEGRADED | 3 d |

## Providers + scorers (`evalkit-providers`, `evalkit-scorers-text`, `evalkit-scorers-llm`)

| # | Delta | Severity | Build |
|---|-------|----------|-------|
| P1 | `CodeAgentAcquisition` — built-in support for Claude Code / Codex / OpenCode / Pi CLIs, declared via `[[matrix.agent]]`. Knows each agent's flag surface, parses each agent's stream-json/session JSONL, fills `OutputTelemetry`. Replaces V1's four `bin/shim-*` Rust crates. | **BLOCKING** | 3 wk |
| P2 | `McpRegistry` — launches stdio/http MCP servers inside `TrialContext.network_ns`, emits per-agent config files (`~/.claude/mcp.json`, `~/.codex/config.toml [mcp_servers.*]`, `~/.local/share/opencode/mcp.json`) into `home_path`. Pi configured to refuse; emits `NotApplicable` combo status. | **BLOCKING** | 1 wk |
| S1 | `TestRunnerScorer` — `[[scorer]] type = "test_runner"`. Runs argv inside `TrialContext.worktree_path` after applying `produce()`'s diff. Framework-agnostic exit-code parse + optional `passed_count` / `total_count` extraction via regex. | **BLOCKING** | 1 wk |
| S2 | `DiffStatsScorer` — `type = "diff_stats"`. `git -C <worktree> diff --numstat HEAD` + glob match against `task.expected_scope`; populates DiffStats. | **BLOCKING** | 3 d |
| S3 | `AgentTelemetryScorer`, `McpTelemetryScorer`, `CostRollupScorer` — `type = "agent_telemetry" | "mcp_telemetry" | "cost_rollup"`. Pure functions over `OutputTelemetry`. No LLM, no shell. | **BLOCKING** | 4 d combined |
| S4 | `RecoveryRateScorer` — `type = "recovery_rate"`. Pure function over tool-call sequence: `(failed_call → corrected_call) / failed_call_count`. | NICE-TO-HAVE | 1 d |
| S5 | `llm_judge` extensions: load rubric from a file path; `both_orderings = true` flag (delegates to K4); per-call `cap_usd`; `round = "screen|focus"` filter. | DEGRADED | 3 d |

## evalkit-cli additions

| # | Delta | Severity | Build |
|---|-------|----------|-------|
| C1 | `[[matrix.agent]]` + `[[matrix.indexer]]` + `[[matrix.editor]]` cartesian expansion. Deterministic combo IDs (`{agent}__{indexer}__{editor}`); auto-status (`Runnable | Degraded | NotApplicable`) from agent capability flags + `[[matrix.exclude]]`. | **BLOCKING** | 1 wk |
| C2 | `[[matrix.exclude]] when = { … }` predicates for structural N/A (Pi-without-MCP, Codex-without-native-edit-disable, etc.). | **BLOCKING** | 2 d |
| C3 | `enabled: bool` (default true) on every row in `[[task]]`, `[[matrix.*]]`, `[[scorer]]`; `--include <id,…>` and `--exclude <id,…>` flags on `evalkit run`. The binding ergonomic constraint of the iterative cadence. | **BLOCKING** | 3 d |
| C4 | Inline `[[task]]` blocks in TOML (today only `[dataset] file = "tasks.jsonl"` is supported). | DEGRADED | 2 d |
| C5 | `[stats]` — `comparison = "paired_bootstrap"`, `correction = "holm_bonferroni"`, `b_iter`, `alpha`. Lives in a new `evalkit-stats` crate or as an `evalkit::stats` module. | **BLOCKING** | 1 wk |
| C6 | `[stats.winner_gate]` — declarative gate evaluation; verdicts go into `summary.json` and `report.md`. | **BLOCKING** | 3 d |
| C7 | Built-in report panels: `[report] tables = ["pareto", "best_combo", "best_per_agent", "preregistered_pairs", "delta_vs_previous"]`. Renders inline-ASCII Pareto by default, SVG when `svg = true`. | **BLOCKING** | 1 wk |
| C8 | `--previous reports/runs/<prev>/summary.json` flag for delta-vs-prior comparison; populates `Summary.delta_vs_previous`. | **BLOCKING** | 2 d |
| C9 | `evalkit validate-mcp --config suite.toml` preflight — probes each MCP server's `tools/list`, surfaces transport / launch / capability errors before a multi-hour run. | NICE-TO-HAVE | 3 d |
| C10 | `[[report.custom]] template = "panel.tera"` templating extension point (Tera or MiniJinja). | NICE-TO-HAVE | 4 d |

## Totals and ordering

- **BLOCKING for v2 UX:** 14 deltas, ~12 engineer-weeks. Required to ship the 80% case as one TOML file.
- **DEGRADED:** 4 deltas, ~2 engineer-weeks. Improves correctness or enforcement but the suite runs without them.
- **NICE-TO-HAVE:** 3 deltas, ~2 engineer-weeks. UX polish.

Suggested build order (pre-suite-launch path):
1. K3 + K4 (3 d + 2 d) — telemetry shape + judge wrapper. Smallest, unblocks scoring work.
2. K1 + K2 (1 wk + 2 wk) — isolation. Required before any code-agent runs untrusted.
3. P2 (1 wk) — MCP launcher + per-agent config emitters.
4. S1 + S2 + S3 (1 wk + 3 d + 4 d) — built-in scorers.
5. P1 (3 wk) — CodeAgentAcquisition. Largest single piece; benefits from the foundation above.
6. C1 + C2 + C3 (1 wk + 2 d + 3 d) — matrix expansion + enable/disable.
7. C5 + C6 + C7 + C8 (1 wk + 3 d + 1 wk + 2 d) — stats, gate, report panels, delta.
8. K5 + K6, S4 + S5, C4, C9 + C10 — tail.

Critical path is P1 (CodeAgentAcquisition) at 3 weeks; everything else either runs in parallel or shorter. With a single engineer, ~3 calendar months to first end-to-end suite run; with two engineers splitting kernel + cli tracks, ~6 weeks.
