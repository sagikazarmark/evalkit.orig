# evalkit Competitive Analysis

## 1. Landscape map

**Python** is the mature ecosystem. Three categories coexist: app-eval libraries that double as test runners (DeepEval, Ragas, Inspect AI, TruLens, agentevals), platform SDKs that require a backend account or self-hosted server (Phoenix, Langfuse, Braintrust Python SDK), and benchmark harnesses for academic model comparison (`lm-evaluation-harness`, OpenAI Evals). Inspect AI is the de facto standard for serious safety evals; Ragas owns the RAG metric vocabulary; DeepEval owns the pytest-style developer ergonomic. All ship with deeply opinionated `LLMTestCase`/`Sample` types and assume a process they can own.

**TypeScript** is shallow but converging fast. `evalite` (Matt Pocock) and Braintrust's JS SDK are the only credible options; both copy Braintrust's `Eval(name, { data, task, scores })` shape. autoevals is the shared scorer library that both consume. The whole ecosystem is one company plus one personal project — and Vercel's AI SDK is the implicit provider abstraction everyone leans on.

**Rust** is effectively empty for app-eval. Searching crates.io yields `llm-test-bench`, `rusty-llm-jury`, and `llm-cascade` — all hobby-scale, none with the `Eval`/`Scorer`/`Dataset` triad and none with traction. Braintrust shipped `braintrust-sdk-rust 0.1.0-alpha.1` (a tracing client, not an eval framework). There is no Inspect AI equivalent in Rust. evalkit has no direct competitor.

**Platforms** (Phoenix, Langfuse, Braintrust, LangSmith) dominate the long tail of "we want a UI". They differ on license (Phoenix Elastic, Langfuse MIT+ee, Braintrust closed) and on whether the SDK is usable without the backend (Phoenix and Braintrust: yes; Langfuse: yes but the value is the UI). The trend in 2025–2026 is converging on OpenInference/OpenTelemetry semantic conventions for traces, which gives evalkit a stable wire-format target.

## 2. Per-library teardowns

### Phoenix (Arize) / `arize-phoenix-evals`

**Positioning.** Open-source observability + eval platform; the `arize-phoenix-evals` Python package is usable standalone without the Phoenix server.

**Core abstractions.** Post-3.0 redesign (April 7, 2026) — the legacy `llm_classify` / `run_evals` / models wrapper subpackage was deleted. Current API is built around `LLM`, `Evaluator`, and dataframe entry points:

```python
from phoenix.evals import LLM, async_evaluate_dataframe
from phoenix.evals.metrics import DocumentRelevanceEvaluator

llm = LLM(provider="openai", model="gpt-4o")
relevance_evaluator = DocumentRelevanceEvaluator(llm=llm)

results_df = await async_evaluate_dataframe(
    dataframe=df,
    evaluators=[relevance_evaluator],
    concurrency=10,
)
```

Source: [arize-phoenix-evals docs](https://arize.com/docs/phoenix/sdk-api-reference/python/arize-phoenix-evals) (accessed 2026-04-23).

- `LLM` adapter (provider + model)
- `Evaluator` (built-in or custom)
- `async_evaluate_dataframe` / sync variant — pandas `DataFrame` is the dataset format
- OpenInference/OTel instrumentation baked into evaluators

**Strengths.**
- Owns the OpenInference semantic conventions — every span is OTLP-valid ([OpenInference spec](https://arize-ai.github.io/openinference/spec/), accessed 2026-04-23).
- The 3.0 redesign collapsed three overlapping APIs (`llm_classify`, `run_evals`, `Evaluator`) into one.
- Provider adapter pattern keeps OpenAI/LiteLLM/LangChain optional rather than required.
- Pandas-native — fits where data scientists already are.

**Weaknesses.**
- DataFrame-shaped APIs are dead on arrival outside Python; not a model evalkit can borrow.
- Two breaking releases in 12 months (v3.0, plus the v14 server CLI/legacy-client cull on the same date — see [release notes](https://arize.com/docs/phoenix/release-notes/04-2026/04-07-2026-phoenix-v14-breaking-changes), accessed 2026-04-23). Stability discipline is poor.
- Couples evaluators to LLM clients at construction time — testing without a real provider is awkward.
- The Phoenix server is the implicit destination; standalone usage is supported but marketed as a stepping stone.

**Relevance to evalkit.**
- Steal the OpenInference span schema for the `evalkit-otel` exporter — do not invent a competing one.
- Reject the dataframe-as-dataset model. evalkit's iterator-shaped `Dataset` is correct.
- Watch the v3.0 evaluator surface as a stable reference for "what a clean evaluator API looks like after three iterations" — but note Phoenix needed a 14-major-version path to get there.

### agentevals (LangChain)

**Positioning.** Single-purpose scorer library for agent trajectories. Not a runner. Sits next to LangSmith for execution.

**Core abstractions.**

```python
from agentevals.trajectory.match import create_trajectory_match_evaluator

evaluator = create_trajectory_match_evaluator(
    trajectory_match_mode="strict"
)
result = evaluator(outputs=outputs, reference_outputs=reference_outputs)

# LLM-as-judge variant
from agentevals.trajectory.llm import (
    create_trajectory_llm_as_judge, TRAJECTORY_ACCURACY_PROMPT
)
trajectory_evaluator = create_trajectory_llm_as_judge(
    prompt=TRAJECTORY_ACCURACY_PROMPT,
    model="openai:o3-mini",
)
```

Source: [agentevals README](https://github.com/langchain-ai/agentevals) (accessed 2026-04-23).

- `create_trajectory_match_evaluator(mode=…)` — strict / unordered / superset / subset
- `create_trajectory_llm_as_judge(prompt, model)`
- Inputs are OpenAI-format message dicts or LangChain `BaseMessage`
- `graph_trajectory_strict_match` for LangGraph state-graph trajectories

**Strengths.**
- Tight scope: scorers only, no harness opinion.
- Forces the right data shape for agent evals (full message trajectory, not just final output).
- Both deterministic and LLM-judge variants ship in one package with the same call signature.

**Weaknesses.**
- Hardcoded to OpenAI message format and LangChain `BaseMessage` — leaks LangChain into the type system.
- Default model wiring goes through `langchain_openai` — installing `agentevals` pulls a chunk of LangChain.
- "create_x_evaluator" factories instead of types is a Python-ism that doesn't translate to typed languages.

**Relevance to evalkit.**
- Steal the trajectory-vs-final-output split as a first-class kernel concept (currently deferred to `evalkit-scorers-agent`).
- Reject the OpenAI-message-dict-or-LangChain-BaseMessage shape. Define a provider-neutral `Trajectory` / `Step` type in the kernel.
- Watch which trajectory match modes (strict/unordered/superset/subset) get traction — that's the API surface to mirror.

### Langfuse

**Positioning.** Open-source LLM engineering *platform* (MIT, except `ee/`). SDK-as-platform-client. Self-hostable.

**Core abstractions.** SDK-side `Eval`-equivalent is `dataset.run_experiment`:

```python
from langfuse import get_client, Evaluation
from langfuse.openai import OpenAI

langfuse = get_client()

def my_task(*, item, **kwargs):
    response = OpenAI().chat.completions.create(
        model="gpt-4", messages=[{"role": "user", "content": item.input}]
    )
    return response.choices[0].message.content

def accuracy_evaluator(*, input, output, expected_output, **kwargs):
    if expected_output and expected_output.lower() in output.lower():
        return Evaluation(name="accuracy", value=1.0)
    return Evaluation(name="accuracy", value=0.0)

dataset = langfuse.get_dataset("my-evaluation-dataset")
result = dataset.run_experiment(
    name="Production Model Test",
    task=my_task,
    evaluators=[accuracy_evaluator],
)
```

Source: [Langfuse experiments-via-SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) (accessed 2026-04-23).

- `Dataset` lives server-side, fetched by name
- `Evaluation { name, value }` return type
- `task` and `evaluator` are kwargs-only callables
- Tracing piggybacks via `langfuse.openai.OpenAI` monkey-wrap

**Strengths.**
- The strongest hosted/self-hosted UX in the OSS landscape — datasets, runs, traces in one UI.
- MIT-licensed core; ee folder is clearly walled off ([Langfuse repo](https://github.com/langfuse/langfuse), accessed 2026-04-23).
- OpenTelemetry ingest, not a proprietary wire format.
- The `dataset.run_experiment` API explicitly auto-traces — minimal user code.

**Weaknesses.**
- The dataset *must* be server-side. There is no in-process dataset abstraction. That's a non-starter for a library that needs to work without a backend.
- Self-hosted SDK compatibility is gated: docs warn against `api.observations` v2 on self-hosted, telling users to fall back to `api.legacy.observations_v1` ([self-hosted v4 discussion](https://github.com/orgs/langfuse/discussions/12926), accessed 2026-04-23). Versioning across SDK and server is a recurring pain.
- Provider observability comes via monkey-patched `langfuse.openai.OpenAI` — magic, not composition.

**Relevance to evalkit.**
- Steal the `Evaluation { name, value }` minimal scorer return shape — closely matches evalkit's `Score`.
- Reject the server-side-dataset assumption. evalkit's `Dataset` must be in-process; Langfuse becomes an `evalkit-exporters-langfuse` target (which the workspace already plans for).
- Watch the OTel ingest path — it's the right integration shape for evalkit.

### Braintrust (SDK + autoevals + platform)

**Positioning.** Closed-source platform; open-source SDKs (JS, Python, Ruby, Rust alpha) and autoevals. The SDK can run evals locally and report to the platform — or just stdout.

**Core abstractions.**

```typescript
import { Eval } from "braintrust";
import { LevenshteinScorer } from "autoevals";

Eval("Say Hi Bot", {
  data: () => [
    { input: "Foo", expected: "Hi Foo" },
    { input: "Bar", expected: "Hello Bar" },
  ],
  task: (input) => "Hi " + input,
  scores: [LevenshteinScorer],
});
```

Source: [braintrust-sdk-javascript README](https://github.com/braintrustdata/braintrust-sdk) (accessed 2026-04-23).

User-defined scorer signature:

```typescript
const exactMatch = (args: { input: string; output: string; expected?: string }) => {
  return { name: "Exact match", score: args.output === args.expected ? 1 : 0 };
};
```

Source: [Write evaluations](https://www.braintrust.dev/docs/platform/experiments/write) (accessed 2026-04-23).

- `Eval(name, { data, task, scores })` is the entire surface
- Scorer = function returning `{ name, score: 0..1 }` (optionally `metadata`)
- autoevals provides drop-in scorers (Factuality, Levenshtein, Battle, etc.)
- Tracing/spans are implicit — `Eval` opens a span; nested model calls auto-link

**Strengths.**
- The smallest credible eval API in the landscape. Three keys, three concepts. This is the design to beat.
- Scorer signature is provider-agnostic — just `{ input, output, expected }` in, `{ name, score }` out.
- autoevals is published independently and consumed by competitors (evalite imports from `autoevals`) — proves the scorer library can be unbundled from the runner.
- Rust SDK exists ([`braintrust-sdk-rust` 0.1.0-alpha.1](https://crates.io/crates/braintrust-sdk-rust/0.1.0-alpha.1), accessed 2026-04-23) — but it's tracing-only, not the `Eval` primitive.

**Weaknesses.**
- The score-must-be-0..1 contract is a soft one — direction (higher-vs-lower better) is undocumented per scorer.
- `Eval` registers itself globally and is invoked by Braintrust's runner CLI (`braintrust eval ...`); calling `Eval()` from a normal program is supported but surprising.
- Platform integration is closed-source; if the company changes pricing, your traces are stranded unless you also wrote your own exporter.

**Relevance to evalkit.**
- Steal the `Eval(name, { data, task, scores })` shape literally. evalkit's `docs/eval-facade.md` already does this — keep it.
- Steal the autoevals split: scorers as a separately-versioned crate, runner as another. evalkit's per-category scorer crates are correct but should be cross-runtime — autoevals is the precedent.
- Reject the implicit-span-on-`Eval` magic for the Rust core. Make tracing an explicit subscriber, not a side effect of constructing the eval.

### Inspect (UK AISI)

**Positioning.** Research-grade eval framework. Owns the process. Built for safety evals, capability evals, and agentic tasks. MIT, Python ≥3.10.

**Core abstractions.**

```python
from inspect_ai import Task, task
from inspect_ai.dataset import Sample
from inspect_ai.scorer import exact
from inspect_ai.solver import generate

@task
def hello_world():
    return Task(
        dataset=[Sample(input="Just reply with Hello World", target="Hello World")],
        solver=[generate()],
        scorer=exact(),
    )
```

Run with `inspect eval hello_world.py`. Source: [Inspect tutorial](https://inspect.aisi.org.uk/tutorial.html) (accessed 2026-04-23).

- `Task(dataset, solver, scorer)` — the four primitives
- `Sample(input, target, metadata)` — dataset row
- `Solver` chain: `chain_of_thought`, `generate`, `self_critique`, `use_tools`, `bridge` for arbitrary functions
- `Scorer` returns a `Score` with value, explanation, metadata
- `inspect eval` CLI is the canonical entry point; the `eval()` Python API exists but is secondary
- Built-in models layer over OpenAI, Anthropic, Google, Bedrock, Together, Ollama, vLLM, HF transformers

**Strengths.**
- The solver chain abstraction is the cleanest model for multi-step elicitation — lets a single eval express CoT + critique + tool use without owning the model loop.
- Sample-level scoring with structured `Score` (value + explanation + metadata) is more honest than a bare float.
- Production-quality: 200+ pre-built evals at [`inspect_evals`](https://github.com/UKGovernmentBEIS/inspect_evals).
- `bridge()` solver explicitly supports treating an external agent as a black box — the right escape hatch.

**Weaknesses.**
- Hard tokio/asyncio + filesystem assumption: `inspect eval` writes `.eval` log files; the CLI is the primary surface; logs are first-class. Not embeddable in a request-handler.
- `@task` decorator is pure Python magic — relies on import-time discovery. Cannot translate to Rust without giving up types.
- Solvers, scorers, and models are all wired through global registries — ergonomic for scripts, hostile to library use.
- `inspect_ai` package owns its own model client layer rather than wrapping a provider abstraction — adding a new provider means upstreaming.

**Relevance to evalkit.**
- Steal the `Sample` shape (input, target, metadata) and the structured `Score` (value + reasoning + metadata) — evalkit's open `Score` enum question in `docs/decisions.md` should resolve toward this.
- Steal the solver-chain idea for the future agent-eval kernel: a `Solver` trait with `chain_of_thought`, `generate`, `critique` as discrete steps that compose. Inspect proves the model.
- Reject the global-registry / decorator / CLI-owns-the-process pattern outright. evalkit must stay embeddable.
- Reject the bundled model client. Keep providers in `evalkit-providers`; never let model wiring leak into the kernel.

### Promptfoo

**Positioning.** YAML-config eval CLI for prompts and red-teaming. Now an [OpenAI-acquired](https://news.crunchbase.com/ma/data-openai-2023-2026-acquisitions-open-source-astral-promptfoo/) project (accessed 2026-04-23). MIT-licensed. Latest 0.120.26.

**Core abstractions.**

```yaml
prompts:
  - 'Convert the following English text to {{language}}: {{input}}'

providers:
  - openai:chat:gpt-5.4
  - anthropic:messages:claude-opus-4-6

tests:
  - vars:
      language: French
      input: Hello world
    assert:
      - type: contains
        value: 'Bonjour le monde'
  - vars:
      language: Spanish
      input: Where is the library?
    assert:
      - type: icontains
        value: 'Dónde está la biblioteca'
```

Source: [Promptfoo getting started](https://www.promptfoo.dev/docs/getting-started/) (accessed 2026-04-23).

- `promptfooconfig.yaml` is the only required artifact
- `assert` types: `contains`, `icontains`, `equals`, `regex`, `llm-rubric`, `g-eval`, `cost`, `latency`, JS/Python custom
- `promptfoo eval` runs everything in parallel and opens a web UI for diffing

**Strengths.**
- YAML-first means non-engineers can author tests; matrix expansion across prompts × providers × vars is automatic.
- Red-team plugins (`financial:sox-compliance`, `model-identification`, etc.) are uncommon and valuable.
- The web-UI diff for comparing runs is the best-in-class triage surface — users adopt Promptfoo for that alone.

**Weaknesses.**
- YAML-as-API ages badly: the `assert.type` enum has accreted dozens of values; nothing is type-checked until runtime.
- Library-mode exists but is undocumented and second-class — Promptfoo owns the process.
- TypeScript/JS internally; Python and Go users get a CLI subprocess wrapper.
- Now owned by OpenAI: long-term governance and provider neutrality are open questions.

**Relevance to evalkit.**
- Steal the matrix-expansion idea: prompts × providers × vars is a useful default for prompt sweeps. Should be a `Dataset` combinator, not a config syntax.
- Steal the diff-runs UX as inspiration for `evalkit-server`'s minimal review UI.
- Reject YAML-as-API. evalkit is a library; the CLI's config format is a serialization of typed Rust structures, not the source of truth.
- Watch the OpenAI acquisition: if Promptfoo becomes the OpenAI-blessed eval CLI, the TypeScript ecosystem could consolidate around it within 12 months.

### DeepEval (discovered, significant)

**Positioning.** Pytest-shaped eval framework for LLM apps. Confident AI is the SaaS sister product.

**Core abstractions.**

```python
from deepeval import assert_test
from deepeval.metrics import AnswerRelevancyMetric
from deepeval.test_case import LLMTestCase

def test_chatbot():
    answer_relevancy_metric = AnswerRelevancyMetric(threshold=0.7)
    test_case = LLMTestCase(
        input="What if these shoes don't fit?",
        actual_output="We offer a 30-day full refund at no extra costs.",
        retrieval_context=["All customers are eligible for a 30 day full refund at no extra costs."],
    )
    assert_test(test_case, [answer_relevancy_metric])
```

Source: [DeepEval README](https://github.com/confident-ai/deepeval) (accessed 2026-04-23).

- `LLMTestCase(input, actual_output, expected_output, retrieval_context, context, tools_called, ...)`
- 50+ built-in metrics (G-Eval, AnswerRelevancy, Faithfulness, Hallucination, ToxicityMetric, …)
- `assert_test` integrates with pytest; `evaluate()` is the standalone form
- LLM judges configurable via `DeepEvalBaseLLM` subclass

**Strengths.**
- Pytest integration is the single biggest DX win — `pytest tests/test_evals.py` is muscle memory.
- The metric catalog is the broadest in the OSS world; G-Eval, DAG, QAG techniques implemented out of the box.
- Custom-LLM wrapping via `DeepEvalBaseLLM` is the right pattern — provider stays at the edge.

**Weaknesses.**
- `LLMTestCase` is a god-object: input, output, expected, retrieval_context, context, tools_called, expected_tools, reasoning. It accreted; it didn't compose.
- Threshold-as-pass/fail (`AnswerRelevancyMetric(threshold=0.7)`) hides the score and forces every metric into a higher-is-better assumption.
- Confident AI cloud upsell is woven through the docs; some metrics phone home unless explicitly disabled.

**Relevance to evalkit.**
- Steal the metric-catalog vocabulary: AnswerRelevancy, Faithfulness, Hallucination, Bias, Toxicity. evalkit's `evalkit-scorers-rag` should adopt these names verbatim — DeepEval and Ragas have already converged the space.
- Reject the god-`LLMTestCase`. evalkit's `Sample` should stay minimal; richer shapes (with retrieval context, tool calls, trajectories) are separate types in `evalkit-multimodal` / `evalkit-scorers-agent`.
- Reject threshold-as-pass-fail in the kernel. Keep `Score` opinion-free; let assertions live in a higher layer.

### Ragas (discovered, significant)

**Positioning.** RAG-specific evaluation library, now expanding into general LLM eval (v0.4.x).

**Core abstractions.** New API around `DiscreteMetric` / `NumericMetric` with explicit `llm_factory`:

```python
metric = DiscreteMetric(
    name="summary_accuracy",
    allowed_values=["accurate", "inaccurate"],
    prompt="""Evaluate if the summary is accurate..."""
)

score = await metric.ascore(
    llm=llm,
    response="The summary of the text is..."
)
```

Source: [Ragas README](https://github.com/explodinggradients/ragas) (accessed 2026-04-23). Latest 0.4.3, January 2026.

- `Metric` types: `DiscreteMetric`, `NumericMetric`, `RankingMetric`
- Built-in RAG metrics: `Faithfulness`, `AnswerRelevancy`, `ContextPrecision`, `ContextRecall`, `NoiseSensitivity`
- `evaluate(dataset, metrics)` is the batch entry point
- `llm_factory` decouples judge from metric

**Strengths.**
- Owns the RAG metric definitions — anyone implementing faithfulness/context-precision will be measured against Ragas.
- The 0.4.x rewrite explicitly separates metric definition from LLM provider via `llm_factory` — clean.
- `DiscreteMetric` with explicit `allowed_values` is the right shape for categorical LLM judges (most other libraries force a float).

**Weaknesses.**
- Two parallel APIs in the wild right now: the legacy `evaluate(dataset, metrics)` and the new `metric.ascore(llm, ...)`. Migrating users have to learn both.
- Async-only on the new path (`ascore`). Sync is a wrapper.
- Tightly coupled to the HF `datasets` library for batch evaluation.

**Relevance to evalkit.**
- Steal the `DiscreteMetric` / `NumericMetric` / `RankingMetric` taxonomy — maps cleanly onto evalkit's `Score::Categorical` / `Score::Metric` / a future `Score::Ranking`.
- Steal the `llm_factory` indirection — the Rust analogue is the `evalkit-providers` `Acquisition` trait. Ragas validates the pattern.
- Adopt Ragas's RAG metric names verbatim in `evalkit-scorers-rag` (which the roadmap already commits to).

### TruLens (discovered, briefly)

**Positioning.** Snowflake-owned ([acquired May 2024 via TruEra](https://www.trulens.org/)) OpenTelemetry-native tracing + eval library. Open source. Python-only practical use.

- `Feedback` class with provider implementations (`feedback.OpenAI`, `feedback.HuggingFace`, `feedback.Cohere`).
- Wraps user app via `TruApp` / `TruChain` / `TruLlama` with feedback functions attached.
- OpenTelemetry-native as of 2026 ([trulens.org](https://www.trulens.org/), accessed 2026-04-23).

**Relevance.** Confirms the OTel-native direction; otherwise out of scope (Snowflake ownership, Python-only). Don't borrow the TruApp wrapper pattern — it's the same global-instrumentation antipattern as Inspect's decorators.

### evalite (discovered, significant for TS positioning)

**Positioning.** Vitest-shaped eval runner for TypeScript. Matt Pocock, MIT.

**Core abstractions.**

```typescript
import { evalite } from "evalite";
import { Levenshtein } from "autoevals";

evalite("My Eval", {
  data: [{ input: "Hello", expected: "Hello World!" }],
  task: async (input) => { return input + " World!"; },
  scorers: [Levenshtein],
});
```

Source: [evalite quickstart](https://www.evalite.dev/quickstart) (accessed 2026-04-23). Latest 0.19.0.

**Strengths.** Identical shape to Braintrust's `Eval`. Caches AI SDK calls. Watch mode + local web UI. Imports scorers from `autoevals` rather than reinventing.

**Weaknesses.** Owns the process via Vitest. Single-maintainer. v0 — breaking changes expected.

**Relevance.** Validates the `Eval(name, { data, task, scorers })` shape across two independent implementations. evalkit's facade should mirror it.

## 3. Feature matrix

| Library | 1. Positioning | 2. Core abstractions | 3. API style | 4. Provider handling | 5. Storage | 6. Scoring | 7. Dataset | 8. Integration | 9. Runtime assumptions | 10. Stability |
|---|---|---|---|---|---|---|---|---|---|---|
| Phoenix evals | Platform + lib | `LLM`, `Evaluator`, dataframe | Async, imperative | Adapter (`LLM(provider=…)`) | DataFrame in / Phoenix server out | Built-in evaluators, async LLM-judge, 0..1 floats | Pandas DataFrame | Embeddable but server-oriented | asyncio, OS, filesystem logs | ⚠️ Two breaking releases / 12mo (v3.0, v14) |
| agentevals | Library | `create_*_evaluator` factories | Sync + async | LangChain chat models default | None (in-memory) | Strict + LLM-judge, message-list trajectory | None — caller passes lists | Embeddable | asyncio, no FS | ⚠️ v0.x; LangChain-tied |
| Langfuse | Platform | `dataset.run_experiment`, `Evaluation` | Async, imperative | `langfuse.openai.OpenAI` monkey-wrap | Server-side dataset, server-side scores | `Evaluation { name, value }`, server-side LLM-judge | Server-only, fetched by name | Server-required | asyncio, network | ✅ MIT; ⚠️ SDK/server compat gates |
| Braintrust | Platform + SDK | `Eval(name, { data, task, scores })` | JS callback / Py async | Provider stays in `task` | Local + Braintrust cloud | Scorer = `(args) → { name, score }`, autoevals catalog | Function returning array | Embeddable + CLI | Node/Python event loop; Rust SDK is tracing-only alpha | ⚠️ Closed platform; SDK MIT |
| Inspect AI | Framework | `@task`, `Task`, `Sample`, `Solver`, `Scorer` | Decorator + chain DSL | Built-in model layer | `.eval` log files | Structured `Score` (value+explanation+metadata), LLM-judge solvers | `Sample` list / `example_dataset()` | Owns the process via `inspect eval` | asyncio, FS, global registries | ✅ Active, MIT, but no 1.0 |
| Promptfoo | CLI / platform | `promptfooconfig.yaml` | Declarative YAML | Provider strings (`openai:chat:gpt-5.4`) | YAML in / SQLite + web UI out | Assert types incl. `llm-rubric`, `g-eval` | YAML inline + CSV/JSONL refs | Owns the process | Node, FS, browser UI | ⚠️ 0.120.x; OpenAI-owned post-acquisition |
| DeepEval | Library + SaaS | `LLMTestCase`, `Metric`, `assert_test` | Pytest decorator | `DeepEvalBaseLLM` subclass | In-memory + Confident AI cloud | 50+ metrics, threshold-pass/fail, LLM-judge | `LLMTestCase` lists | Embedded in pytest | asyncio, FS | ⚠️ Cloud upsell in core |
| Ragas | Library | `Metric` (Discrete/Numeric/Ranking), `evaluate()` | Async, imperative | `llm_factory` indirection | HF `datasets` in / DataFrame out | Categorical + numeric + ranking, LLM-judge default | HF `datasets` | Embeddable | asyncio | ⚠️ v0.4.x rewrite — two APIs in flight |
| TruLens | Library + platform | `TruApp` wrapper, `Feedback` | Decorator-ish wrapping | `feedback.OpenAI`, etc. | SQLite + OTel | `Feedback` functions, LLM-judge providers | User-supplied | Wraps the app | asyncio, OTel, FS | ✅ Active; Snowflake-owned |
| OpenAI Evals | Benchmark harness | YAML registry + `Eval` class | YAML + Python | OpenAI client baked in | Registry on disk | Model-graded YAML, custom Python | YAML + JSONL | Owns the process | Python, FS, OpenAI key | ⚠️ Maintenance-only ([last meaningful commit Nov 2025](https://github.com/openai/evals/commits/main), accessed 2026-04-23; 2026 commits are CI/pre-commit pins) |
| evalite | Library | `evalite(name, { data, task, scorers })` | Vitest-shaped | Caller's choice in `task` | Vitest + local UI | `autoevals` import | Inline array | Owns the process via Vitest | Node | ⚠️ v0.19 single-maintainer |
| lm-evaluation-harness | Benchmark harness | Tasks registry | YAML + CLI | HF, vLLM, OpenAI, etc. | FS results | Standardized benchmark scorers | Standard benchmarks (MMLU, etc.) | Owns the process | Python, FS | ✅ Stable but academic-scope |
| **evalkit** | **Library + optional CLI** | **`Eval`, `Scorer`, `Dataset`, `Sample`, `Run`, `Score`** | **Imperative, async** | **`Acquisition` trait + `evalkit-providers`** | **In-memory; sinks pluggable** | **`Score` enum (value/categorical/metric); LLM-judge in `evalkit-scorers-llm`** | **In-process iterator-shaped** | **Embeddable, no `main()` ownership** | **No tokio lock-in goal; getrandom/js feature for wasm32** | **Pre-1.0 (0.3.0); kernel-boundary plan in flight** |

## 4. Gap analysis for evalkit

### Table-stakes features evalkit is missing

**P0 (block 1.0):**
- **LLM-as-judge scorer in the kernel API contract.** Every competitor ships this; evalkit defers to `evalkit-scorers-llm`. Fine — but the kernel's `ScorerContext` must carry whatever a judge needs (sample id, run id, attempt index, structured metadata). Goal #1 (stable API) — once this leaks out post-1.0 it's a breaking change.
- **Structured `Score` variant.** Inspect, Ragas, and the Braintrust scorer return shape all carry `{ value, reasoning/explanation, metadata }`. evalkit's open decision in `docs/decisions.md` should resolve to `Structured { score: f64, reasoning: String, metadata: Value }` (Inspect's shape). Goal #1 + Goal #2.
- **Trajectory / message-shaped Sample.** agentevals, Inspect, DeepEval all support multi-step trajectories; evalkit currently doesn't. Defer the *scorers* to `evalkit-scorers-agent`, but the kernel must define the `Trajectory` and `Step` types now or break later. Goal #1.
- **Score direction semantics.** Higher-is-better vs lower-is-better is undocumented across the field — and evalkit can fix it cheaply with a `direction: ScoreDirection` field on `Score::Metric`. Goal #1 + Goal #2.

**P1 (block adoption):**
- **`Eval(name, { data, task, scores })` happy-path facade in TS/Python bindings.** The Rust `Eval` exists ([commit `190c01a`](Eval happy-path facade in recent git log)); the bindings need to expose the same shape. Goal #2 + Goal #5.
- **A `bridge`-equivalent solver.** Inspect's `bridge()` lets users plug an arbitrary callable as a solver. evalkit needs the equivalent: a way to wrap an existing app as a `Task` without rewriting it. Goal #5.
- **OpenInference span emission.** Phoenix and TruLens already emit it; if evalkit doesn't, integrators will write the conversion themselves. Goal #4 + #5.
- **Streaming/online scoring path.** The roadmap calls this "Rust's production-tier advantage" but it's also table stakes — Langfuse, Phoenix, Braintrust all score live traces. Goal #5.

**P2 (post-1.0):**
- **Web UI for run diffing.** Promptfoo's diff is best in class; evalkit-server can ship a minimal version later.
- **Matrix expansion** (prompts × providers × vars) as a `Dataset` combinator.
- **Pre-built scorer catalog** matching the DeepEval/Ragas vocabulary. Backlog already exists in `docs/scorers.md`.
- **Red-team plugin family** (Promptfoo's lead).

### Deliberate differentiators evalkit has that the landscape validates

- **Library-first, doesn't own `main()`.** Inspect, Promptfoo, evalite, and DeepEval all own the process. Phoenix/Langfuse/Braintrust SDKs are embeddable but optimized for their backend. None are designed to drop into a long-running Rust service handling requests. evalkit has the field to itself. Goal #5.
- **Provider isolation via `Acquisition` trait + `evalkit-providers`.** Ragas's `llm_factory` and DeepEval's `DeepEvalBaseLLM` validate the pattern; Inspect's bundled model layer is the cautionary tale. Goal #4.
- **Workspace split with no kernel deps on tokio runtime.** No competitor has even tried WASM. Validated by absence — it's a real gap. Goal #3.
- **Open `Score` taxonomy** (Categorical / Metric / Pass-Fail / Structured) over the autoevals `0..1` float convention. Ragas's `DiscreteMetric` / `NumericMetric` / `RankingMetric` proves the field is moving toward this. Goal #1.

### Deliberate differentiators the landscape challenges

**Steelman first, then verdict.**

- **Polyglot via subprocess plugin protocol** (Phase 1 in evalkit's roadmap).
  - *Steelman.* Lets Python/TS users keep their scorer libraries (Ragas, autoevals, DeepEval) without porting. Avoids reinventing 50+ metrics. Lets evalkit be the runner without owning the metric catalog.
  - *Counter.* Subprocess IPC kills the streaming/online use case (latency); every competitor that tried plugin protocols has had them wither (OpenAI Evals' custom-code submissions are now closed). Most users will adopt the language they already use; polyglot becomes a feature nobody asks for at runtime.
  - *Verdict.* Keep the protocol but downgrade its priority. Ship native scorers in `evalkit-scorers-*` first; polyglot is Phase 1, not Phase 0. The roadmap order is right.

- **No bundled provider client; everything via `Acquisition`.**
  - *Steelman.* Inspect bundles its own client because it lets the framework guarantee retries, timeouts, log capture, and concurrency limits — things every eval needs. Composing those across an external `Acquisition` trait is harder.
  - *Counter.* Bundling a client is exactly what locks Inspect's models layer into Python and prevents WASM. evalkit's `Acquisition` trait can require the implementer to handle retries/timeouts; evalkit can publish a default `RetryAcquisition<A>` decorator.
  - *Verdict.* Keep the trait. Ship `RetryAcquisition` and `TimeoutAcquisition` decorators in `evalkit-providers` so the ergonomic gap closes.

- **Library-first with optional CLI.**
  - *Steelman.* Most users start by running `inspect eval`, `promptfoo eval`, `pytest`, or `vitest` — a CLI is the discovery surface. Library-first means a slower hello-world.
  - *Counter.* Library-first is the only way to get embedded use cases (in-process scoring inside a Rust web server) and WASM. The CLI is a shell over the library; users who want CLI-first can still get it.
  - *Verdict.* Hold the line. Make sure `evalkit-cli`'s hello-world is *as good* as Promptfoo's `init` — that's the discovery deficit to close.

### Cloudflare Workers / WASM

**Who supports it.** Nobody in the eval landscape. Phoenix, Langfuse, Braintrust SDKs, Inspect, Promptfoo, DeepEval, Ragas, agentevals, evalite, TruLens — all assume Node, asyncio, or tokio + filesystem. There is no competitor to displace; there is also no proof the use case has demand outside one project (evalkit's own goal).

**Who assumes tokio + OS.** All of them. Specifically:
- Inspect/DeepEval/Ragas/Phoenix/Langfuse/TruLens — Python asyncio, FS log writes, signal handlers.
- Braintrust SDK / evalite / Promptfoo — Node FS + child processes.
- `braintrust-sdk-rust` — pulls `tokio` features by default ([crate page](https://crates.io/crates/braintrust-sdk-rust/0.1.0-alpha.1), accessed 2026-04-23).

**Patterns evalkit must reject to preserve `wasm32-unknown-unknown` compatibility:**
- **No `tokio::fs`, `std::fs`, or any filesystem assumption in the kernel.** Sinks accept `AsyncWrite`/`Write` that callers provide; the kernel never opens a file.
- **No `tokio::net`, `reqwest` defaults, or any sockets in the kernel.** HTTP belongs in `evalkit-providers`, not in `evalkit`.
- **No `tokio::time::sleep`, `Instant::now()` without `chrono`'s `wasmbind` feature.** evalkit already pins `getrandom` with `js` feature for wasm32 in the root `Cargo.toml` — extend the discipline to time.
- **No `tokio::spawn`.** Use `futures::stream::FuturesUnordered` for concurrency; let the runtime drive. The Cloudflare Workers runtime invokes handlers via `spawn_local` on a single-threaded JS event loop — `Send` bounds break it ([cloudflare/workers-rs docs](https://developers.cloudflare.com/workers/languages/rust/), accessed 2026-04-23).
- **No `mio`, `async-std`, `smol`, `signal-hook` transitive deps.** mio fails to build for wasm32-unknown-unknown ([workers-rs#736](https://github.com/cloudflare/workers-rs/issues/736), accessed 2026-04-23).
- **No global state for configuration** (registries, lazy_statics holding clients). Workers re-instantiate per request; globals corrupt.
- **No thread-local storage** for context propagation. Use explicit `&ScorerContext` parameters.
- **No `lazy_static`/`once_cell` holding non-`Send` types.**
- **No `std::time::SystemTime::now()` without target gating.**
- **No default `Send + Sync + 'static` bounds on `Future`/`Stream` types in the kernel** — they over-constrain the worker case.

The cost of holding this line is paid once. The cost of breaking it after 1.0 is paid forever.

### Provider isolation

**Who leaks providers into the core:**
- **Inspect AI** — `inspect_ai.model` ships clients for ~12 providers; adding one means upstreaming. Worst offender.
- **agentevals** — defaults to `langchain_openai`; importing the package pulls it.
- **DeepEval** — `LLMTestCase` is provider-neutral, but the metrics default to OpenAI judges; `DeepEvalBaseLLM` exists but is opt-in.
- **TruLens** — `feedback.OpenAI`, `feedback.Cohere`, `feedback.HuggingFace` are first-class submodules; switching providers means switching imports.
- **Langfuse** — `langfuse.openai.OpenAI` monkey-wrap pattern; works only for OpenAI clients out of the box.

**Who keeps providers isolated:**
- **Braintrust** — `task` is a user callable; provider lives entirely in user code. Cleanest in the field.
- **Phoenix v3.0** — `LLM(provider=…, model=…)` adapter, evaluators take `llm` at construction. Clean boundary.
- **Ragas v0.4.x** — `llm_factory` indirection; metric definition has no provider knowledge.
- **evalite** — `task` is a callback; provider is user's problem. Same as Braintrust.

**Pattern that wins.** Two-layer isolation. The kernel never names a provider; user code calls the model inside `task`/`Acquisition`. Where the kernel needs a model (LLM-as-judge scorers), inject via a trait at construction (`Phoenix LLM`, `Ragas llm_factory`, evalkit's `Acquisition`).

evalkit's current design — `Acquisition` trait in `evalkit-providers`, scorers parameterized over it — is correct. Don't relax it. The temptation will be to add a "convenience" `evalkit::default_openai_judge()` at some point; reject it. That's how Inspect ended up with `inspect_ai.model`.

## 5. High-level feature recommendations

1. **Lock the `Score` enum and `ScorerContext` shape now.** Adopt Inspect's `Structured { value, reasoning, metadata }` for the LLM-judge case; add `direction: ScoreDirection` to `Score::Metric`; populate `ScorerContext` with `{ sample_id, run_id, attempt, metadata }`. Why: Goal #1 (stable API) — these surfaces leak through every scorer crate. Scope: small. Prior art: Inspect's `Score`, Ragas's `DiscreteMetric`/`NumericMetric` taxonomy.

2. **Ship the `Eval(name, { data, task, scorers })` facade as the documented hello-world in Rust, TS, and Python.** Why: Goal #2 (DX) + Goal #5 (easy to integrate). Scope: small in Rust (already in `190c01a`); medium for the bindings. Prior art: Braintrust SDK + evalite — copy the call shape verbatim, including the `scores`/`scorers` keyword.

3. **Define `Trajectory` and `Step` kernel types now, before agent scorers land.** Why: Goal #1 — adding these later is a breaking change to `Sample`. Scope: small. Prior art: agentevals' OpenAI-message-list shape (without LangChain-coupling) and Inspect's `messages` field on `Sample`.

4. **Publish OpenInference-conformant spans from `evalkit-otel`.** Why: Goal #5 — Phoenix, TruLens, and Envoy AI Gateway already emit OpenInference; conforming makes evalkit immediately usable inside any of those backends. Scope: medium. Prior art: OpenInference spec at [arize-ai.github.io/openinference/spec](https://arize-ai.github.io/openinference/spec/) (accessed 2026-04-23). Don't invent a competing schema.

5. **Add `RetryAcquisition<A>` and `TimeoutAcquisition<A>` decorators in `evalkit-providers`.** Why: Goal #4 — closes the ergonomic gap that motivates competitors to bundle their own clients. Scope: small. Prior art: Inspect's bundled model layer (the failure mode); the Tower middleware pattern (the right shape).

6. **`bridge`-style adapter solver for embedding existing apps.** Why: Goal #5 — the use case is "I have a Rust web service, score its responses inline" without rewriting. Scope: small. Prior art: Inspect AI's `bridge()`. Make the Rust version explicit: `Task::from_fn(|sample| async { ... })`.

7. **WASM CI job.** Cargo-build the kernel + `evalkit-runtime` (sans tokio-net features) + `evalkit-providers` (HTTP backend gated off) for `wasm32-unknown-unknown` on every PR. Why: Goal #3 — without CI enforcement the assumption rots inside a release. Scope: small. Prior art: workers-rs project layout for feature-gating.

8. **Adopt Ragas + DeepEval metric naming in `evalkit-scorers-rag` and `evalkit-scorers-llm`.** `faithfulness`, `answer_relevancy`, `context_precision`, `context_recall`, `noise_sensitivity`, `hallucination`, `bias`, `toxicity`, `g_eval`. Why: Goal #2 — users transferring from Python should not have to re-learn the vocabulary. Scope: medium (per-scorer implementation work, not API). Prior art: Ragas, DeepEval — both stable on these names.

9. **Streaming/online scoring with a `PullExecutor` driven by an external future.** Why: Goal #3 + Goal #5 — Cloudflare Worker request handlers cannot spawn tasks; they need to drive scoring from their own poll loop. Scope: medium. Prior art: nothing in the eval space — design from first principles, not a competitor.

10. **Run-log JSONL schema with semver-independent versioning.** Why: Goal #1 — the run log is the polyglot interop surface; if it shifts under users, the bindings story collapses. Scope: medium. Prior art: Inspect's `.eval` log file (closed binary format — do not adopt); HuggingFace `datasets` JSONL/Parquet conventions. Pick JSONL for human-greppable, allow Parquet for scale (`docs/ROADMAP.md` already plans this).

## 6. Miscellaneous

- **Naming collision.** `evalkit` on **crates.io** is *your own* publication: v0.1.0 by `sagikazarmark`, published 2026-04-03 ([crates.io API check](https://crates.io/crates/evalkit), accessed 2026-04-23). On **PyPI** the name is taken — `evalkit 0.2.1` ("EVolutionary ALgorithms KITs") by user `parvector`, last released 2021 ([pypi.org/project/evalkit](https://pypi.org/project/evalkit/), accessed 2026-04-23). Practically dead but the name is locked; you'll need a different PyPI package name (`evalkit-py`, `pyeval-kit`, etc.) or a take-over request via PEP 541. **npm**: name registration unverified (page returned 403 to fetch); check `npm view evalkit` directly before TS bindings ship. **No collision on crates.io for `evalkit-*` extension crate names** — workspace strategy is safe.

- **OpenInference / OpenTelemetry conventions.** `openinference.span.kind` is the required attribute; valid values cover `LLM`, `CHAIN`, `RETRIEVER`, `RERANKER`, `EMBEDDING`, `AGENT`, `TOOL`, `EVALUATOR`, `GUARDRAIL` ([OpenInference semantic conventions](https://arize-ai.github.io/openinference/spec/semantic_conventions.html), accessed 2026-04-23). Use `EVALUATOR` for scorer spans. The OpenTelemetry GenAI semantic conventions ([opentelemetry.io/docs/specs/semconv/gen-ai](https://opentelemetry.io/docs/specs/semconv/gen-ai/), accessed 2026-04-23) are upstream-stabilizing in parallel — track both, emit OpenInference today, migrate when OTel GenAI hits stable.

- **Dataset format standards.** JSONL is the practical lingua franca (every competitor reads it). Parquet is HuggingFace's auto-conversion target for hub datasets; supporting it via `datafusion` or `arrow2` is the right path for scale. HuggingFace `datasets` library wraps both. Recommendation: JSONL is the kernel format; Parquet is an `evalkit-providers` feature.

- **Trace/span conventions.** Don't invent. Adopt OpenInference for spans; for the run log, JSONL with one record per `Sample`/`Score`/`Run` event keyed by an `event_type` discriminator. Inspect's `.eval` is binary — don't copy it.

- **Agent eval vs model eval split.** This is the load-bearing axis the field is converging on. Model evals = single-shot input → output (Ragas, DeepEval, Phoenix). Agent evals = trajectory (agentevals, Inspect, LangSmith, Goodeye Labs roundup at [goodeyelabs.com top tools 2026](https://www.goodeyelabs.com/articles/top-ai-agent-evaluation-tools-2026), accessed 2026-04-23). evalkit's kernel must support both shapes natively (`Sample` for the former, `Trajectory` for the latter); split the *scorers* into separate crates as already planned.

- **License considerations.** All open-source competitors are MIT or Apache-2.0 except Phoenix server (Elastic 2.0 with carve-outs) and Langfuse `ee/` (commercial). evalkit's MIT/Apache-2.0 dual is the safe default; Cloudflare/AWS-friendly downstream consumers care about Apache-2.0 patent grant.

- **OpenAI's eval-tooling acquisitions.** OpenAI acquired Promptfoo in early 2026 ([Crunchbase M&A roundup](https://news.crunchbase.com/ma/data-openai-2023-2026-acquisitions-open-source-astral-promptfoo/), accessed 2026-04-23). If OpenAI consolidates eval tooling under one brand within 12 months, the TS landscape may collapse to "OpenAI eval CLI" + "Braintrust" — making evalkit's Rust + WASM + library-first niche *more* defensible, not less.

- **Stability signal worth flagging.** `openai/evals` repository is in maintenance-only mode: 2026 commits are CI hygiene (pre-commit hook pinning, GH Actions ref pinning); the last substantive change was November 2025's `incontext_rl` removal ([commits view](https://github.com/openai/evals/commits/main), accessed 2026-04-23). Treat it as historical reference, not active competition.
