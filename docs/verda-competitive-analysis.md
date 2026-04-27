# Landscape map

> **Note:** The planning and execution recommendations in this document are superseded by `docs/evalkit-kernel-boundary-plan.md`.
>
> This file still stands as market and competitive research. Use the newer document for the current kernel-boundary plan, sequencing, and verification bar.

The current eval tooling market falls into four practical groups. First are platform-centric systems such as Phoenix, Langfuse, and Braintrust: they combine tracing, datasets, experiments, storage, and UI, and they treat evals as one workflow inside a larger product surface. Second are Python frameworks such as Inspect, DeepEval, and Ragas: these expose substantial in-process abstractions and built-in metrics, but they usually assume Python runtimes, event loops, filesystem access, or framework-owned workflows. Third are config- and CLI-first tools such as Promptfoo and OpenAI Evals: effective for CI, benchmark runs, and prompt comparisons, but centered on declarative config rather than a small embeddable kernel. Fourth are benchmark harnesses such as `lm-evaluation-harness`: powerful for model comparison, but not designed as low-level app-embedded eval libraries. Phoenix positions itself as observability plus evaluation plus experiments, Langfuse as open-source LLM engineering, Braintrust as platform plus SDK, Inspect as a framework for LLM evaluations, Promptfoo as CLI and library for evals and red teaming, and OpenAI Evals as a framework plus registry while OpenAI now emphasizes hosted dashboard evals. [Phoenix README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md), [Phoenix eval quickstart](https://arize.com/docs/phoenix/get-started/get-started-evaluations), [Langfuse README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md), [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md), [Inspect docs](https://inspect.aisi.org.uk/), [Promptfoo README](https://raw.githubusercontent.com/promptfoo/promptfoo/main/README.md), [OpenAI Evals README](https://raw.githubusercontent.com/openai/evals/main/README.md) (all accessed 2026-04-23).

For `verda`, that split matters more than raw feature count. The market already has many experiment runners and review surfaces. What it does not clearly have is a broadly adopted, stable, low-level, Rust-native eval kernel that is library-first, provider-isolated, and plausibly portable to Workers or WASM. The current implementation already points in that direction: semver discipline in the kernel, explicit schema/versioning work, provider-neutral scorer design, and a roadmap that prefers integrating with observability platforms rather than becoming one. That is a real opening, but only if the core stays small and resists inheriting the runtime, storage, and product assumptions that dominate the rest of the landscape. Local refs: `docs/ROADMAP.md`, `docs/stability.md`, `src/lib.rs`, `examples/basic.rs`, `examples/prod_eval_daemon.rs`.

# Per-library teardowns

## Phoenix (Arize)

1. Positioning
Phoenix is an observability-first platform that also ships evals, datasets, experiments, prompt tooling, and tracing. It is not a minimal eval kernel. [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md), [eval quickstart](https://arize.com/docs/phoenix/get-started/get-started-evaluations) (accessed 2026-04-23).

2. Core abstractions
The practical building blocks are traces/spans, evaluators, datasets, experiments, and annotations. The docs consistently route evals through existing trace or dataset data rather than through a small standalone `Eval` object. [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md), [eval quickstart](https://arize.com/docs/phoenix/get-started/get-started-evaluations) (accessed 2026-04-23).

3. API style
Python SDK and TS packages exist, but the TS eval package is explicitly alpha. The API is reasonably ergonomic, but it assumes Phoenix concepts and often Phoenix persistence. [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md) (accessed 2026-04-23).

```python
from phoenix.evals import LLM
from phoenix.evals import ClassificationEvaluator

llm = LLM(model="gpt-4o", provider="openai")

completeness_evaluator = ClassificationEvaluator(
    name="completeness",
    prompt_template=financial_completeness_template,
    llm=llm,
    choices={"complete": 1.0, "incomplete": 0.0},
)
```

Source: [Phoenix eval quickstart](https://arize.com/docs/phoenix/get-started/get-started-evaluations) (accessed 2026-04-23).

4. Provider/model handling
Provider support is broad, mostly through OpenInference/OpenTelemetry instrumentation and vendor integrations. That is good coverage, but it is platform-mediated rather than a strict provider-isolation boundary in a tiny core. [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md) (accessed 2026-04-23).

5. Storage/persistence
Strongly storage-aware. Evaluations attach to traces and annotations in Phoenix. That is useful product behavior, but it is the opposite of a storage-agnostic kernel. [eval quickstart](https://arize.com/docs/phoenix/get-started/get-started-evaluations) (accessed 2026-04-23).

6. Scoring model
LLM-as-judge and code evaluators are both supported. The example classification evaluator maps labels to numeric values, which is a practical shape for comparisons. [eval quickstart](https://arize.com/docs/phoenix/get-started/get-started-evaluations) (accessed 2026-04-23).

7. Dataset model
Phoenix has versioned datasets and experiments tied to those datasets. Good for iterative workflows; heavier than `verda` needs in core. [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md) (accessed 2026-04-23).

8. Integration story
Excellent if the team already wants OTel/OpenInference and a Phoenix deployment. Weak if the goal is “just embed a tiny typed library.” [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python or TS SDKs plus Phoenix-compatible tracing/storage workflows. Not Workers/WASM-friendly in the low-level sense. [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md) (accessed 2026-04-23).

10. Stability signals
Active releases and broad docs surface are strong signals; TS evals being alpha is an explicit caution. Latest GitHub release observed was `arize-phoenix-v14.11.0` on 2026-04-22. [GitHub releases](https://github.com/Arize-ai/phoenix/releases), [README](https://raw.githubusercontent.com/Arize-ai/phoenix/main/README.md) (accessed 2026-04-23).

Bottom line
Phoenix is a strong observability-plus-evals product. It validates export and OTel integration for `verda`; it does not validate moving storage, UI, and trace semantics into `verda` core.

## agent-evals

1. Positioning
`agent-evals` is a collection of evaluation scripts for agents, not a mature reusable library. The repository is archived. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md), [GitHub repo](https://github.com/langchain-ai/agent-evals) (accessed 2026-04-23).

2. Core abstractions
There are effectively no durable library abstractions exposed in the README beyond per-eval folders with `README.md` and `run_eval.py`. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

3. API style
Script-first and repo-structured, not package-first. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

```text
Each folder in the repo contains:

- `README.md`: A description of the evaluation
- `run_eval.py`: A script to run the evaluation
```

Source: [agent-evals README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Not clearly abstracted in the repo overview. Distribution/package state outside GitHub was blocked during research, so package status is `[UNCERTAIN]`. Local research note plus [GitHub repo](https://github.com/langchain-ai/agent-evals) (accessed 2026-04-23).

5. Storage/persistence
Datasets are linked out to LangSmith. That suggests dependence on external hosted dataset artifacts rather than a self-contained model. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

6. Scoring model
Task-specific scripts rather than a consistent scorer interface. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

7. Dataset model
Hosted dataset links, no visible typed local dataset abstraction. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

8. Integration story
Useful as examples of agent tasks, weak as a foundation for others to embed. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python scripts. No sign of portability ambitions. [README](https://raw.githubusercontent.com/langchain-ai/agent-evals/main/README.md) (accessed 2026-04-23).

10. Stability signals
Poor. The repo is archived and has no published GitHub releases. [GitHub repo](https://github.com/langchain-ai/agent-evals) (accessed 2026-04-23).

Bottom line
`agent-evals` is mostly evidence that agent benchmarks/scripts are easy to publish and hard to generalize. For `verda`, the lesson is to avoid repo-of-scripts drift and keep first-class abstractions explicit.

## Langfuse

1. Positioning
Langfuse is an open-source LLM engineering platform with tracing, datasets, experiments, scoring, and prompt management. [README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md), [evaluation overview](https://langfuse.com/docs/evaluation/overview) (accessed 2026-04-23).

2. Core abstractions
Observations/traces, scores, datasets, dataset runs, experiments, prompt objects. The core data model is product-shaped, not kernel-shaped. [README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md), [experiments via SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) (accessed 2026-04-23).

3. API style
Python and JS SDKs are clean. The experiment runner is especially usable because it takes `data`, `task`, and optional evaluators, but the surrounding platform concepts remain visible. [experiments via SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) (accessed 2026-04-23).

```python
result = langfuse.run_experiment(
    name="Geography Quiz",
    description="Testing basic functionality",
    data=local_data,
    task=my_task,
)
```

Source: [Langfuse experiments via SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) (accessed 2026-04-23).

4. Provider/model handling
Strong integration coverage through SDK wrappers and OTel. Good operationally, but provider isolation is not the design center; instrumentation and hosted tracing are. [README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md), [instrumentation docs](https://langfuse.com/docs/observability/sdk/instrumentation) (accessed 2026-04-23).

5. Storage/persistence
Persistence is first-class. Hosted or self-hosted Langfuse stores traces, scores, datasets, and experiment runs. [README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md), [datasets docs](https://langfuse.com/docs/evaluation/dataset-runs/datasets) (accessed 2026-04-23).

6. Scoring model
Supports numeric, categorical, boolean, and text scores, plus custom and LLM-as-judge evaluations. That is one of the cleaner score models among platform tools. [scores overview](https://langfuse.com/docs/evaluation/scores/overview), [LLM-as-a-judge](https://langfuse.com/docs/evaluation/evaluation-methods/llm-as-a-judge) (accessed 2026-04-23).

7. Dataset model
Datasets and dataset runs are central, with versioning built in. [datasets docs](https://langfuse.com/docs/evaluation/dataset-runs/datasets), [experiments via SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) (accessed 2026-04-23).

8. Integration story
Excellent if a team wants observability and evals in one place. Less compelling as a small dependency-free core. [README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Normal server-side Python/JS plus OTel. Not a Workers/WASM-native shape. [README](https://raw.githubusercontent.com/langfuse/langfuse/main/README.md), [experiments via SDK](https://langfuse.com/docs/evaluation/experiments/experiments-via-sdk) (accessed 2026-04-23).

10. Stability signals
Strong. High release cadence and mature docs. Latest GitHub release observed was `v3.169.0` on 2026-04-17. [GitHub releases](https://github.com/langfuse/langfuse/releases) (accessed 2026-04-23).

Bottom line
Langfuse validates the value of experiments, dataset versioning, and multi-type scores. It argues for `verda` exporters and adapters, not for copying Langfuse’s storage-bound architecture into core.

## Braintrust (SDK + Autoevals + platform)

1. Positioning
Braintrust is a platform with a notably ergonomic SDK for evals and logging. Autoevals is the lighter-weight scorer library orbiting that platform. [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md), [Autoevals README](https://raw.githubusercontent.com/braintrustdata/autoevals/main/README.md), [Braintrust docs](https://www.braintrust.dev/docs/evaluation) (accessed 2026-04-23).

2. Core abstractions
The SDK centers an `Eval` object with `data`, `task`, and `scores`. Autoevals centers `{input, output, expected}` scorers. This is the cleanest top-level abstraction in the market and the strongest direct inspiration for `verda` ergonomics. [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md), [Autoevals README](https://raw.githubusercontent.com/braintrustdata/autoevals/main/README.md) (accessed 2026-04-23).

3. API style
Very good. Concise, obvious, low ceremony at the call site. [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md) (accessed 2026-04-23).

```python
from autoevals import LevenshteinScorer
from braintrust import Eval

Eval(
    "Say Hi Bot",
    data=lambda: [
        {"input": "Foo", "expected": "Hi Foo"},
        {"input": "Bar", "expected": "Hello Bar"},
    ],
    task=lambda input: "Hi " + input,
    scores=[LevenshteinScorer],
)
```

Source: [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md) (accessed 2026-04-23).

```typescript
import { Factuality } from "autoevals";

const result = await Factuality({ output, expected, input });
```

Source: [Autoevals README](https://raw.githubusercontent.com/braintrustdata/autoevals/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Autoevals supports OpenAI-compatible APIs and can use Braintrust’s proxy/gateway. That gives breadth, but also introduces Braintrust as a control plane in the happy path. [Autoevals README](https://raw.githubusercontent.com/braintrustdata/autoevals/main/README.md), [proxy docs](https://www.braintrust.dev/docs/guides/proxy) (accessed 2026-04-23).

5. Storage/persistence
Platform-first. The SDK can operate with local/logging controls such as `no_send_logs`, but the product value clearly expects Braintrust storage and comparison workflows. [Braintrust docs](https://www.braintrust.dev/docs/guides/evals), [Python SDK reference](https://www.braintrust.dev/docs/reference/sdks/python) (accessed 2026-04-23).

6. Scoring model
Good range of heuristic, statistical, embedding, and LLM-based scorers in Autoevals. The normalized score interface is simple. [Autoevals README](https://raw.githubusercontent.com/braintrustdata/autoevals/main/README.md) (accessed 2026-04-23).

7. Dataset model
Simple datasets work well in the SDK; the platform adds experiment comparison. The dataset model is adequate, but the conceptual emphasis is still on experiment runs. [Braintrust docs](https://www.braintrust.dev/docs/evaluation) (accessed 2026-04-23).

8. Integration story
Strongest in class for teams comfortable with Braintrust. For `verda`, this is the main DX benchmark to match. [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python and TS, server-side, networked. Not designed for Workers/WASM. [Braintrust SDK README](https://raw.githubusercontent.com/braintrustdata/braintrust-sdk-python/main/README.md), [Autoevals README](https://raw.githubusercontent.com/braintrustdata/autoevals/main/README.md) (accessed 2026-04-23).

10. Stability signals
Strong release cadence. Latest GitHub releases observed were `py-sdk-v0.16.0` for the Python SDK on 2026-04-17 and `py-0.2.0` for Autoevals on 2026-04-02. [braintrust-sdk-python releases](https://github.com/braintrustdata/braintrust-sdk-python/releases), [autoevals releases](https://github.com/braintrustdata/autoevals/releases) (accessed 2026-04-23).

Bottom line
Braintrust is the strongest positive ergonomic reference for `verda`. The caution is not to copy its platform coupling, proxy assumptions, or broader control-plane leakage into the kernel.

## Inspect (UK AI Security Institute)

1. Positioning
Inspect is a framework for large language model evaluations. It is explicit about being a framework, not just a metric pack. [Inspect docs](https://inspect.aisi.org.uk/), [README](https://raw.githubusercontent.com/UKGovernmentBEIS/inspect_ai/main/README.md) (accessed 2026-04-23).

2. Core abstractions
`Task`, datasets, solvers, scorers, tools, agents, logs. This is the closest conceptual match to `verda`’s `Task` and `Scorer` orientation. [Inspect docs](https://inspect.aisi.org.uk/) (accessed 2026-04-23).

3. API style
Readable and structured, but heavier than Braintrust because tasks often compose multiple framework concepts. [Inspect docs](https://inspect.aisi.org.uk/) (accessed 2026-04-23).

```python
from inspect_ai import Task, task
from inspect_ai.dataset import example_dataset
from inspect_ai.scorer import model_graded_fact
from inspect_ai.solver import chain_of_thought, generate, self_critique

@task
def theory_of_mind():
    return Task(
        dataset=example_dataset("theory_of_mind"),
        solver=[chain_of_thought(), generate(), self_critique()],
        scorer=model_graded_fact()
    )
```

Source: [Inspect docs](https://inspect.aisi.org.uk/) (accessed 2026-04-23).

4. Provider/model handling
Broad provider support through provider strings like `openai/gpt-4o`, plus many local and hosted options, including Cloudflare in provider docs. [Inspect docs](https://inspect.aisi.org.uk/), [providers docs](https://inspect.aisi.org.uk/providers.html) (accessed 2026-04-23).

5. Storage/persistence
Logs are written to `./logs` by default and viewed with `inspect view`. This is a reasonable framework default and a bad kernel default for `verda`. [Inspect docs](https://inspect.aisi.org.uk/) (accessed 2026-04-23).

6. Scoring model
Supports text comparison, model grading, and custom scorers. Strong for structured pipelines. [scorers docs](https://inspect.aisi.org.uk/scorers.html) (accessed 2026-04-23).

7. Dataset model
Datasets are a first-class task component and are flexible enough for many sources. [datasets docs](https://inspect.aisi.org.uk/datasets.html) (accessed 2026-04-23).

8. Integration story
Strong for users who want the whole framework, including tools and agent evaluation. Weaker if they want a tiny library they can bend to their own runtime. [Inspect docs](https://inspect.aisi.org.uk/) (accessed 2026-04-23).

9. Runtime assumptions
Python framework, async architecture, filesystem logs, optional UI/tooling, and substantial execution environment assumptions. Not Workers/WASM-friendly. [Inspect docs](https://inspect.aisi.org.uk/) (accessed 2026-04-23).

10. Stability signals
Very active. The changelog shows rapid 0.3.x iteration; latest visible changelog entry during research was `0.3.210` on 2026-04-22. [CHANGELOG](https://inspect.aisi.org.uk/CHANGELOG.html) (accessed 2026-04-23).

Bottom line
Inspect strongly validates `Task`/pipeline structure and disciplined composition. It also shows exactly what `verda` should not pull into core: tools, logs, sandboxes, and framework-owned runtime policy.

## Promptfoo

1. Positioning
Promptfoo is a CLI and library for evaluating and red-teaming LLM apps. [README](https://raw.githubusercontent.com/promptfoo/promptfoo/main/README.md), [docs intro](https://www.promptfoo.dev/docs/intro/) (accessed 2026-04-23).

2. Core abstractions
Prompts, providers, tests, assertions, transforms, and red-team configs. Strong declarative configuration story. [configuration reference](https://www.promptfoo.dev/docs/configuration/reference/), [node package docs](https://www.promptfoo.dev/docs/usage/node-package/) (accessed 2026-04-23).

3. API style
Primarily YAML/CLI, with a Node API for programmatic use. The Node API is clean but still mirrors the config model. [node package docs](https://www.promptfoo.dev/docs/usage/node-package/) (accessed 2026-04-23).

```typescript
import promptfoo from 'promptfoo';
const results = await promptfoo.evaluate(testSuite, options);
```

Source: [Promptfoo node package docs](https://www.promptfoo.dev/docs/usage/node-package/) (accessed 2026-04-23).

4. Provider/model handling
Very broad via provider strings and provider functions. Good for evaluation breadth, but centered on JS runtime/provider adapters. [README](https://raw.githubusercontent.com/promptfoo/promptfoo/main/README.md), [providers docs](https://www.promptfoo.dev/docs/providers/) (accessed 2026-04-23).

5. Storage/persistence
Local-first with disk writes, view commands, and optional sharing/cloud. Good DX, but not kernel-neutral. [README](https://raw.githubusercontent.com/promptfoo/promptfoo/main/README.md), [node package docs](https://www.promptfoo.dev/docs/usage/node-package/) (accessed 2026-04-23).

6. Scoring model
Assertions are the main unit, including JS assertions and named scores. Strong practical CI behavior, less elegant as a reusable typed scorer model. [node package docs](https://www.promptfoo.dev/docs/usage/node-package/) (accessed 2026-04-23).

7. Dataset model
Test cases and vars are adequate, but less explicit than dedicated dataset/case types. [configuration reference](https://www.promptfoo.dev/docs/configuration/reference/) (accessed 2026-04-23).

8. Integration story
Excellent for CI, prompt comparison, and security testing. Less suitable as the low-level core of another system. [README](https://raw.githubusercontent.com/promptfoo/promptfoo/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Node, CLI, local files, optional cloud/self-hosting. Not Workers/WASM-first. [README](https://raw.githubusercontent.com/promptfoo/promptfoo/main/README.md), [node package docs](https://www.promptfoo.dev/docs/usage/node-package/) (accessed 2026-04-23).

10. Stability signals
Strong activity and docs. Latest GitHub release observed was `0.121.7` on 2026-04-22. [GitHub releases](https://github.com/promptfoo/promptfoo/releases) (accessed 2026-04-23).

Bottom line
Promptfoo is a strong example of DX and CI pragmatism. `verda` should borrow the ease of use, not the config-first center of gravity.

## DeepEval

1. Positioning
DeepEval is a Python evaluation framework positioned as “Pytest for LLM apps.” [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

2. Core abstractions
Metrics, test cases, datasets, tracing/observe decorators, CLI test runner, platform integration. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

3. API style
Usable, but framework-heavy. The pytest analogy is real: nice in Python, not portable outside it. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

```python
from deepeval import assert_test
from deepeval.metrics import GEval
from deepeval.test_case import LLMTestCase, LLMTestCaseParams

correctness_metric = GEval(
    name="Correctness",
    criteria="Determine if the 'actual output' is correct based on the 'expected output'.",
    evaluation_params=[LLMTestCaseParams.ACTUAL_OUTPUT, LLMTestCaseParams.EXPECTED_OUTPUT],
    threshold=0.5,
)
```

Source: [DeepEval README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Broad; docs emphasize “ANY LLM,” local NLP models, and framework integrations. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

5. Storage/persistence
Can run locally, but platform integration with Confident AI is a major part of the story. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

6. Scoring model
Huge metric catalog, including agentic, RAG, multi-turn, multimodal, and MCP-specific metrics. That is breadth, not necessarily a clean minimal core. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

7. Dataset model
`EvaluationDataset` and generated goldens are first-class. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

8. Integration story
Very good for Python-first teams that want batteries included. Weak fit for `verda`’s library-first portable kernel. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python, CLI, environment variables, local file loading, networked judges. Not portable to Workers/WASM. [README](https://raw.githubusercontent.com/confident-ai/deepeval/main/README.md) (accessed 2026-04-23).

10. Stability signals
Active, though the surface area is large and fast-moving. Latest GitHub release observed was `v3.9.7` on 2025-12-01. [GitHub releases](https://github.com/confident-ai/deepeval/releases) (accessed 2026-04-23).

Bottom line
DeepEval is a feature catalog and workflow framework. `verda` should not compete on metric breadth in core.

## Ragas

1. Positioning
Ragas is an evaluation and test-generation toolkit for LLM apps, especially RAG. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

2. Core abstractions
Metrics, LLM wrappers, quickstart templates, and test generation. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

3. API style
Clean enough, but narrower than general-purpose frameworks. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

```python
metric = DiscreteMetric(
    name="summary_accuracy",
    allowed_values=["accurate", "inaccurate"],
    prompt="""Evaluate if the summary is accurate and captures key information.

Response: {response}

Answer with only 'accurate' or 'inaccurate'."""
)

score = await metric.ascore(llm=llm, response="The summary of the text is...")
```

Source: [Ragas README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Uses explicit LLM wrappers and works with major frameworks/observability tooling. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

5. Storage/persistence
Primarily library-level, but not aggressively minimal. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

6. Scoring model
RAG-focused and metric-oriented. Good vocabulary reference; not a universal score kernel. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

7. Dataset model
Strong on test generation and production-aligned test sets. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

8. Integration story
Useful as a metric vocabulary and adapter target for `verda`, especially via a dedicated RAG scorer crate. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python and LLM-backed scoring. No portability story for Workers/WASM. [README](https://raw.githubusercontent.com/vibrantlabsai/ragas/main/README.md) (accessed 2026-04-23).

10. Stability signals
Active. Latest GitHub release observed was `v0.4.3` on 2026-01-13. [GitHub releases](https://github.com/vibrantlabsai/ragas/releases) (accessed 2026-04-23).

Bottom line
Ragas is narrower and more reusable than DeepEval, but still Python-first. For `verda`, it is more useful as naming and scorer inspiration than as a structural competitor.

## TruLens

1. Positioning
TruLens is instrumentation plus evaluation plus tracking, with a strong dashboard and OTel story. [README](https://raw.githubusercontent.com/truera/trulens/main/README.md), [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

2. Core abstractions
`Metric`, `Selector`, `TruApp`, `TruSession`, dashboard, providers. [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

3. API style
Expressive, but opinionated and instrumentation-heavy. [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

```python
from trulens.core import Metric
from trulens.core import Selector

f_answer_relevance = Metric(
    implementation=provider.relevance_with_cot_reasons,
    name="Answer Relevance",
    selectors={
        "prompt": Selector.select_record_input(),
        "response": Selector.select_record_output(),
    },
)
```

Source: [TruLens quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

4. Provider/model handling
Broad provider support, with OTel and provider modules as part of the architecture. [README](https://raw.githubusercontent.com/truera/trulens/main/README.md), [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

5. Storage/persistence
Session/database/dashboard are first-class concepts. This is not a storage-agnostic kernel. [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

6. Scoring model
Metrics plus selectors are powerful, especially for RAG and instrumentation-derived evaluation. [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

7. Dataset model
Supports ground truth datasets and feedback flows, but the center is still instrumentation over app traces. [README](https://raw.githubusercontent.com/truera/trulens/main/README.md) (accessed 2026-04-23).

8. Integration story
Good if the team wants tracing and evaluation together. Less compelling as a tiny embedded core. [README](https://raw.githubusercontent.com/truera/trulens/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python, dashboards, DB/session setup, OTel. Not Workers/WASM-oriented. [quickstart](https://www.trulens.org/getting_started/quickstarts/quickstart/) (accessed 2026-04-23).

10. Stability signals
Active. Latest GitHub release observed was `2.7.2` on 2026-04-09. [GitHub releases](https://github.com/truera/trulens/releases) (accessed 2026-04-23).

Bottom line
TruLens is another strong signal that OTel-aware exporters matter. It is not evidence that `verda` core should own sessions, selectors, or dashboards.

## OpenAI Evals

1. Positioning
OpenAI Evals remains an OSS framework and registry, but the README now explicitly points users to dashboard-based Evals in the OpenAI platform. [README](https://raw.githubusercontent.com/openai/evals/main/README.md), [OpenAI platform evals guide](https://platform.openai.com/docs/guides/evals) (accessed 2026-04-23).

2. Core abstractions
Registry, templates, completion functions, YAML/JSONL-defined evals, plus some custom-code paths. [README](https://raw.githubusercontent.com/openai/evals/main/README.md), [eval templates docs](https://raw.githubusercontent.com/openai/evals/main/docs/eval-templates.md) (accessed 2026-04-23).

3. API style
Config- and registry-oriented. Good for standardized eval authoring; not great for embedding a tiny library in an app runtime. [README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

```sh
pip install evals
```

```sh
git lfs fetch --all
git lfs pull
```

Source: [OpenAI Evals README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Historically OpenAI-centered, with completion function extensibility. Not provider-isolation-first. [README](https://raw.githubusercontent.com/openai/evals/main/README.md), [completion-fns docs](https://raw.githubusercontent.com/openai/evals/main/docs/completion-fns.md) (accessed 2026-04-23).

5. Storage/persistence
Registry and optional Snowflake logging. [README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

6. Scoring model
Template-driven and model-graded patterns are the main reusable contribution. [README](https://raw.githubusercontent.com/openai/evals/main/README.md), [build eval docs](https://raw.githubusercontent.com/openai/evals/main/docs/build-eval.md) (accessed 2026-04-23).

7. Dataset model
JSON/YAML registry datasets with Git LFS. Reasonable for benchmark publication, awkward for lightweight embedding. [README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

8. Integration story
Useful as an import/export target and for prompt-template ideas. Weak fit as `verda`’s model. [README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python, Git LFS, registry assets, OpenAI-oriented workflows. Not portable. [README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

10. Stability signals
Mixed. High historical importance, but the hosted platform now appears to be the strategic center. GitHub API did not report a latest release during research. [README](https://raw.githubusercontent.com/openai/evals/main/README.md) (accessed 2026-04-23).

Bottom line
OpenAI Evals remains influential as a template and registry design, not as the right model for a low-level Rust library.

## lm-evaluation-harness

1. Positioning
The EleutherAI harness is a benchmark runner for model evaluation across many tasks and many backends. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

2. Core abstractions
Tasks, model backends, CLI/config, benchmark outputs. It is benchmark-first, not app-eval-core-first. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

3. API style
Mostly CLI/config driven, with some Python API usage. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

```bash
lm_eval --model hf \
    --model_args pretrained=EleutherAI/gpt-j-6B \
    --tasks hellaswag \
    --device cuda:0 \
    --batch_size 8
```

Source: [lm-evaluation-harness README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Extremely broad across local runtimes, inference servers, and commercial APIs. This is one of its major strengths. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

5. Storage/persistence
Output paths, cache directories, Hub/W&B/Zeno integrations. Practical, but not minimal. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

6. Scoring model
Task-specific benchmark scoring, not a small general scorer abstraction. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

7. Dataset model
Benchmark task registry, not app-centric datasets. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

8. Integration story
Important adjacent tool. Not a direct substitute for `verda`. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Python, often GPU-heavy, filesystem and CLI oriented. No Workers/WASM fit. [README](https://raw.githubusercontent.com/EleutherAI/lm-evaluation-harness/main/README.md) (accessed 2026-04-23).

10. Stability signals
Strong and long-lived. Latest GitHub release observed was `v0.4.11` on 2026-02-13. [GitHub releases](https://github.com/EleutherAI/lm-evaluation-harness/releases) (accessed 2026-04-23).

Bottom line
This is the wrong product category for `verda` to compete with directly. The useful takeaway is benchmark import/export, not architecture.

## Evalite

1. Positioning
Evalite is a TypeScript-native, local-first eval tool. [quickstart](https://www.evalite.dev/quickstart/), [README](https://raw.githubusercontent.com/mattpocock/evalite/main/readme.md) (accessed 2026-04-23).

2. Core abstractions
`evalite(name, { data, task, scorers })`, file conventions, local UI, traces, SQLite-backed local results. [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

3. API style
Extremely approachable for TS users. This is one of the better examples of “small top-level API with strong local feedback.” [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

```typescript
import { evalite } from "evalite";
import { Levenshtein } from "autoevals";

evalite("My Eval", {
  data: [{ input: "Hello", expected: "Hello World!" }],
  task: async (input) => {
    return input + " World!";
  },
  scorers: [Levenshtein],
});
```

Source: [Evalite quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

4. Provider/model handling
Typically delegated to whatever TS AI stack the user already has; docs highlight Vercel AI SDK integration. [quickstart](https://www.evalite.dev/quickstart/), [AI SDK example](https://www.evalite.dev/examples/ai-sdk/) (accessed 2026-04-23).

5. Storage/persistence
Saves results to SQLite under `node_modules/.evalite` and runs a local UI. Good DX, bad fit for a portable core. [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

6. Scoring model
Scorers are simple and often borrowed from Autoevals. Effective, if not especially principled. [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

7. Dataset model
Plain arrays and function-returned data. Simple and good. [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

8. Integration story
Strong with modern TS, especially local iteration loops. [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

9. Runtime assumptions
Node, Vitest, SQLite bindings, local UI. Not Workers/WASM-friendly. [quickstart](https://www.evalite.dev/quickstart/) (accessed 2026-04-23).

10. Stability signals
Good docs and clear product direction. Latest GitHub release observed was `evalite@0.19.0` on 2025-11-06. [GitHub releases](https://github.com/mattpocock/evalite/releases) (accessed 2026-04-23).

Bottom line
Evalite is a strong DX benchmark for local-first TS workflows. The lesson for `verda` is not SQLite or a local UI in core; it is top-level API clarity.

## Rust ecosystem

1. Positioning
The general Rust AI eval ecosystem is still effectively thin. The notable current entrant found in research was `evals` plus `cargo-evals` inside `leostera/agents`, and it is explicitly tied to typed agent systems. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md), [docs.rs evals](https://docs.rs/evals/0.3.0/evals/), [docs.rs cargo-evals](https://docs.rs/cargo-evals/0.3.0/cargo_evals/) (accessed 2026-04-23).

2. Core abstractions
Suites, evals, trajectories, predicates, judges, `cargo evals` discovery. Good for agent testing, not a generic low-level eval substrate. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

3. API style
Plain Rust code plus macros/attributes and cargo subcommands. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

```rust
#[suite(
    kind = "regression", 
    agent = new_agent
)]
async fn new_agent(ctx: EvalContext<()>) -> Result<StringyAgent> {
    Ok(SessionAgent::builder()
        .with_llm_runner(ctx.llm_runner())
        .build()?)
}
```

Source: [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

4. Provider/model handling
Tied to the surrounding `agents` runtime and target config. Not a clean provider-agnostic eval kernel. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md), [docs.rs evals](https://docs.rs/evals/0.3.0/evals/) (accessed 2026-04-23).

5. Storage/persistence
Artifacts under `.evals/` and cargo-driven workflows. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

6. Scoring model
Predicates and judges over trajectories. That is specific and useful, but not general enough to stand in for `verda`’s core. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

7. Dataset model
Trajectory-centric rather than broad dataset/case abstractions. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

8. Integration story
Good if the user already bought into that agent runtime. Weak as a standalone interop layer. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md) (accessed 2026-04-23).

9. Runtime assumptions
Tokio, cargo subcommands, terminal tooling, and non-WASM-friendly dependencies. That makes it a poor fit for Workers/WASM goals. [docs.rs cargo-evals](https://docs.rs/cargo-evals/0.3.0/cargo_evals/) (accessed 2026-04-23).

10. Stability signals
Interesting but early. The ecosystem is sparse enough that `verda` can plausibly define the category if the API stays tight.

Bottom line
Rust is the main opening. The field is thin enough that `verda` does not need to out-feature Python incumbents; it needs to become the obvious embeddable Rust core.

## `verda`

1. Positioning
`verda` is a Rust-native evaluation kernel and toolkit with a library-first roadmap, explicit non-goals around becoming a hosted SaaS, and an architecture that prefers integrating with observability platforms instead of competing with them. Local refs: `docs/ROADMAP.md`, `docs/stability.md`, `docs/gap-analysis.md`.

2. Core abstractions
The public exports include `Dataset`, `Run`, `Scorer`, `ScorerContext`, `Score`, `ScorerSet`, `PullExecutor`, `SampleSource`, `ExecutionSink`, comparison/stats types, and sample shapes. Local ref: `src/lib.rs`.

3. API style
The examples are straightforward and already fairly close to the Braintrust/Inspect blend the project wants: simple top-level runs, explicit scorers, and opt-in executor/runtime layers. Local refs: `examples/basic.rs`, `examples/prod_eval_daemon.rs`.

```rust
let run = Run::builder()
    .dataset(dataset)
    .source(my_source)
    .scorer(ExactMatchScorer)
    .scorer(ContainsReferenceScorer)
    .trials(3)
    .sample_timeout(Duration::from_secs(5))
    .build()?;

let result = run.execute().await?;
```

Source: local ref `examples/basic.rs`.

4. Provider/model handling
The roadmap and decisions explicitly favor provider-neutral seams, including `anyllm::ChatProvider` for LLM judging in the scorer crate rather than provider-specific clients. Local refs: `docs/decisions.md`, `docs/integrations.md`.

5. Storage/persistence
The kernel is semver-anchored and separate from server/exporter crates; persistence concerns are pushed outward. Local refs: `docs/stability.md`, `docs/ROADMAP.md`.

6. Scoring model
The kernel already supports structured scores, metadata, composition operators, token/cost accounting hooks, and run statistics. Local refs: `docs/decisions.md`, `docs/gap-analysis.md`.

7. Dataset model
Datasets and samples are explicit; roadmap work also calls out adapters for Promptfoo, OpenAI Evals, Inspect, Ragas, and others. Local ref: `docs/integrations.md`.

8. Integration story
Strong. Separate crates exist or are planned for providers, OTel, exporters, scorers, CLI, and server. Local refs: `Cargo.toml`, `docs/ROADMAP.md`, `docs/gap-analysis.md`.

9. Runtime assumptions
This is the main tension. The core crate currently depends on `tokio` and the examples use `#[tokio::main]`, so `verda` is not yet fully aligned with a strict “Workers/WASM-safe kernel” story. Local ref: `Cargo.toml`, `examples/basic.rs`, `examples/prod_eval_daemon.rs`.

10. Stability signals
Good internal discipline: explicit decisions log, stability policy, schema versioning, and a 0.2.0 kernel release in the local manifest. Local refs: `Cargo.toml`, `docs/stability.md`, `docs/decisions.md`.

Bottom line
The shape is promising and differentiated. The largest risk is allowing runtime and app-surface conveniences to dilute the kernel boundary before `verda` establishes itself.

# Feature matrix

| Library | Positioning | Core abstractions | API style | Provider/model handling | Storage/persistence | Scoring model | Dataset model | Integration story | Runtime assumptions | Stability signals |
|---|---|---|---|---|---|---|---|---|---|---|
| Phoenix | Observability platform with evals | Traces, evaluators, datasets, experiments, annotations | SDK + platform UI | Broad via OpenInference/OTel; platform-mediated | Phoenix-managed traces/annotations/datasets | LLM judge + code evaluators | Versioned datasets | Excellent with OTel ecosystems | Python/TS + Phoenix workflows | Strong release cadence; TS evals alpha |
| agent-evals | Archived repo of eval scripts | Folder-per-eval scripts | Script-first | `[UNCERTAIN]`; not clearly abstracted | External dataset links via LangSmith | Task-specific scripts | Hosted dataset links | Weak as reusable library | Python scripts | Poor; archived, no releases |
| Langfuse | LLM engineering platform | Traces, scores, datasets, experiments, prompts | Clean Python/JS SDKs | Broad integrations; OTel-centric | Hosted/self-hosted first-class persistence | Numeric/categorical/boolean/text scores | Versioned datasets + dataset runs | Excellent if adopting Langfuse | Server-side Python/JS + OTel | Strong and fast-moving |
| Braintrust + Autoevals | Platform plus ergonomic SDK plus scorer lib | `Eval`, experiments, `{input,output,expected}` scorers | Best-in-class top-level ergonomics | OpenAI-compatible APIs plus Braintrust proxy/gateway | Platform-first with local/logging controls | Heuristic/statistical/LLM scorers | Simple SDK datasets + platform experiments | Strong if okay with Braintrust orbit | Python/TS, networked | Strong release cadence |
| Inspect | Python eval framework | `Task`, datasets, solvers, scorers, tools, logs | Structured framework API | Broad provider strings and local/hosted options | Filesystem logs + viewer | Text, judge, custom scorers | First-class datasets | Strong for full-framework users | Python async + logs + tools | Very active 0.3.x cadence |
| Promptfoo | CLI/library for evals + red teaming | Prompts, tests, providers, assertions, transforms | YAML/CLI with Node API | Very broad via provider strings/functions | Local writes + viewer + optional cloud/share | Assertion-centric, practical CI scoring | Test cases/vars rather than typed datasets | Excellent for CI and app security | Node + CLI + local files | Strong activity |
| DeepEval | Python framework, “pytest for LLM apps” | Metrics, test cases, datasets, tracing, CLI | Pythonic but framework-heavy | Broad, “ANY LLM” messaging | Local plus Confident AI platform tie-in | Very broad metric catalog | `EvaluationDataset`, generated goldens | Strong in Python teams | Python + CLI + env vars | Active, large surface |
| Ragas | RAG-focused eval + test generation toolkit | Metrics, test generation, LLM wrappers | Moderate, narrower than frameworks | Major model/framework integrations | Mostly library-level | RAG-centric metric set | Strong on generated test sets | Good as metric vocabulary target | Python + LLM-backed scoring | Active |
| TruLens | Instrumentation + eval + tracking | `Metric`, `Selector`, `TruApp`, `TruSession` | Expressive, instrumentation-heavy | Broad provider modules + OTel | Session/database/dashboard first-class | Metric + selector model | Ground truth support, trace-centric | Strong with tracing workflows | Python + DB/dashboard/OTel | Active |
| OpenAI Evals | Registry/framework; hosted dashboard now emphasized | Registry, templates, completion functions | Config/template first | OpenAI-centered with extension hooks | Registry assets + optional Snowflake | Template/model-graded patterns | JSON/YAML registry datasets | Good import/export target, weak core model | Python + Git LFS + registry | Mixed; strategic center appears hosted |
| lm-evaluation-harness | Benchmark harness | Tasks, model backends, CLI/config | CLI/config first | Extremely broad backend support | Output paths, caches, Hub/W&B/Zeno | Benchmark/task-specific scoring | Benchmark registries | Important adjacent tool, not direct substitute | Python, often GPU-heavy | Strong, long-lived |
| Evalite | TS-native local-first eval tool | `evalite`, scorers, traces, local UI | Very approachable | Delegated to existing TS stacks | SQLite in `node_modules/.evalite` | Simple scorer arrays | Plain arrays/functions | Strong for local TS iteration | Node + Vitest + SQLite + local UI | Good docs and clear direction |
| Rust `evals` / `cargo-evals` | Agent-runtime-specific Rust evals | Suites, trajectories, predicates, judges | Rust macros + cargo subcommands | Bound to `agents` runtime | `.evals/` artifacts | Predicate/judge over trajectories | Trajectory-centric | Good inside that runtime only | Tokio/cargo/terminal tooling | Early but notable |
| `verda` | Rust-native library-first eval kernel/toolkit | Dataset, Run, Scorer, Score, Executor, schema, stats | Straightforward builder APIs | Explicitly pushing provider-neutral seams | Kernel separated from exporters/server | Structured scores + composition + stats | Explicit datasets/samples with adapter roadmap | Strong crate-split story | Currently still carries Tokio assumptions | Good internal discipline, pre-1.0 |

# Gap analysis for `verda`

## Table-stakes missing or still soft

1. The portable kernel story is not fully convincing yet.
The project goal emphasizes Cloudflare Workers/WASM support, but the current crate still depends on `tokio` and the public examples are Tokio entrypoints. That does not make the design wrong, but it does make the messaging ahead of the implementation. Local refs: `Cargo.toml`, `examples/basic.rs`, `examples/prod_eval_daemon.rs`.

2. Dataset and format interoperability are still mostly roadmap items.
The local backlog explicitly calls out loaders/adapters for OpenAI Evals, Inspect, Promptfoo, Ragas, Hugging Face, SQL, and more. Until some of those land, `verda` remains easier to embed than to adopt from other ecosystems. Local ref: `docs/integrations.md`.

3. The scorer catalog is still thin relative to user expectations set by Python competitors.
The local scorer backlog is large; many canonical text, RAG, and agent scorers are still todo. `verda` does not need to match DeepEval breadth, but it probably needs a more credible “first 20 scorers” story. Local refs: `docs/scorers.md`, `docs/gap-analysis.md`.

4. The top-level naming still needs the `verda` transition completed and explained.
The user context says the project was renamed from `perf` to avoid collision and is now `verda`, while some internal crate/package names in the codebase still reflect the older `evalkit` naming. That ambiguity is operationally manageable but product-costly if left unresolved. Local refs: `Cargo.toml`, user-provided project context.

## Differentiators that the market validates

1. A stable, typed core is genuinely differentiated.
Most competitors either lead with a platform, a Python framework, or a CLI/config workflow. Very few lead with a small semver-disciplined eval kernel. Local refs: `docs/stability.md`, `docs/decisions.md`.

2. Provider isolation is a real gap in the market.
Many tools support many providers, but they do so through gateways, instrumentation layers, or framework-owned provider adapters. That is not the same as keeping provider types out of the eval kernel. Phoenix, Langfuse, Braintrust, Promptfoo, and TruLens all validate the value of broad provider support; they do not remove the need for a strict kernel boundary.

3. Library-first is a meaningful strategic choice.
Promptfoo, Evalite, and Inspect all show that opinionated workflows are useful. They also show how quickly a runtime, viewer, or filesystem becomes mandatory once it enters the center of the design. `verda` can win by keeping CLI/server surfaces optional.

4. Structured pipelines and scorer composition are worth keeping.
Inspect validates `Task` and structured pipelines. Braintrust validates simple `data/task/scores` ergonomics. `verda` already has scorer composition operators and executor/source/sink separation, which is a good synthesis of those influences. Local refs: `docs/decisions.md`, `docs/gap-analysis.md`, `src/lib.rs`.

## Differentiators that are challenged or need discipline

1. “Workers/WASM support” is aspirational until the core actually sheds runtime assumptions.
This is the biggest challenged differentiator. If the core continues to require Tokio-flavored assumptions or indirect filesystem/process dependencies, the market claim will ring hollow.

2. “Good DX” is not yet won just because the internals are clean.
Braintrust, Promptfoo, and Evalite all set a high bar for the first five minutes. `verda`’s current examples are decent, but the path from zero to first useful eval likely still has too much Rust-specific setup compared with the best competitors.

3. “Low-level” can easily become “too much assembly required.”
The market shows two opposite failures: platform bloat and script sprawl. `verda` should avoid both by offering a tiny happy path without forcing users into the full executor/server/exporter world.

## Cloudflare Workers / WASM comparison

1. Phoenix, Langfuse, Braintrust, Inspect, DeepEval, Ragas, TruLens, OpenAI Evals, and `lm-evaluation-harness` are effectively non-starters for a true low-level Workers/WASM core because they assume Python or Node server runtimes, hosted persistence, or heavyweight local execution models.
2. Promptfoo and Evalite are better on DX but still rely on Node, filesystem access, local viewers, or SQLite.
3. Rust `evals` / `cargo-evals` is not a fit because it is agent-runtime-specific and not WASM-friendly.
4. `verda` is the only analyzed project with a plausible path to a real Workers/WASM-friendly eval kernel, but only if the kernel boundary is enforced more strictly than it is today.

## Provider isolation comparison

1. Braintrust and Autoevals have good provider coverage, but Braintrust’s proxy and platform remain part of the gravitational pull.
2. Phoenix and Langfuse are broad but trace/platform-centered.
3. Promptfoo provides wide provider access through provider strings and JS functions, but it does not give a language-neutral typed isolation boundary.
4. Inspect supports many providers cleanly at the framework level, but not as a tiny embeddable kernel.
5. `verda`’s explicit decision to use provider-neutral seams in scorer/provider crates rather than leak provider clients into the kernel is the right differentiator to preserve. Local ref: `docs/decisions.md`.

## What this means for roadmap choices

1. Do not compete head-on with Phoenix/Langfuse/Braintrust as a platform.
2. Do not compete head-on with DeepEval/Ragas on metric catalog breadth in core.
3. Do become the easiest way for a Rust team to embed typed evals without buying a platform.
4. Do make exporters, dataset adapters, and scorer packs the way `verda` meets the rest of the market.

# High-level feature recommendations

1. Make the kernel unambiguously runtime-light.
Remove or quarantine Tokio, filesystem, process, and OS-bound assumptions from the core crate. If needed, split a `verda-runtime` or executor crate from the semver-critical kernel. This is the highest-leverage move because it directly supports the Workers/WASM claim and sharpens the product boundary.

2. Freeze and simplify the top-level API around a tiny happy path.
The best market ergonomics today are Braintrust’s `Eval(data, task, scores)` and Evalite’s `evalite(name, { data, task, scorers })`. `verda` should keep the richer internal model, but offer a comparably obvious front door over `Eval`, `Dataset`, `Task`, `Case`, and `Scorer`.

3. Ship a focused scorer pack before broadening further.
Prioritize a small canonical set: exact/contains/regex/json-schema, Levenshtein ratio, answer similarity, factuality, classifier, G-Eval, pairwise/battle, basic RAG metrics, and a minimal agent/tool trajectory pack. Competing on breadth with DeepEval is a trap; competing on “the obvious essentials are here” is enough.

4. Land import/export adapters for adjacent ecosystems.
The backlog already points the right way. High-value adapters are OpenAI Evals data, Promptfoo config/tests, Inspect task samples, Ragas-like RAG datasets, and exporters for Langfuse/Phoenix/Braintrust. This lets `verda` integrate into existing shops without pretending they will rewrite everything.

5. Keep provider-specific logic outside the kernel and document that boundary aggressively.
This is one of the few design choices the current market genuinely leaves open. Treat provider isolation as a product feature, not just an implementation detail.

6. Keep CLI and server optional and obviously secondary.
The local roadmap already leans this way. Preserve it. The moment `verda` starts requiring a local DB, log directory, or review server for the normal path, it loses its clearest differentiation.

7. Invest in examples that prove the product thesis.
The project already has good examples. Add examples specifically for: pure in-memory local use, judge-backed scoring through provider-neutral traits, exporting to Langfuse/Phoenix, and a constrained runtime example that demonstrates the intended path toward Workers/WASM.

# Miscellaneous

1. Rust ecosystem state
The Rust ecosystem for general-purpose AI eval libraries is still sparse enough that `verda` can plausibly become the category default if it stays disciplined. The main notable current Rust entrant found in live research was the agent-specific `evals`/`cargo-evals` tooling in `leostera/agents`, which is useful but not a direct substitute. [agents README](https://raw.githubusercontent.com/leostera/agents/main/README.md), [docs.rs evals](https://docs.rs/evals/0.3.0/evals/) (accessed 2026-04-23).

2. Newer entrants
I did not find a new general-purpose entrant from the last six months that materially changes the roadmap calculus. The most notable “new” movement is continued acceleration inside existing vendors and the thin-but-interesting Rust `evals` effort. This is partly `[UNCERTAIN]` because package indexes and some anti-bot protections limited exhaustive discovery outside major repos.

3. Recommended competitive stance
Describe `verda` as the stable Rust eval kernel that composes with Braintrust-style ergonomics, Inspect-style structured pipelines, and platform exporters, without becoming a platform itself.

4. Naming note
This analysis treats the project name as `verda`. If the rename is real and imminent, prioritize finishing it across crates, docs, and examples quickly; otherwise product language and internal code/package naming will continue to diverge.

5. Research limitations
`agent-evals` distribution metadata outside GitHub was blocked during live research; those package-status claims remain `[UNCERTAIN]`. Some large docs pages were truncated by tooling during collection, but the claims used here were limited to directly verified content from fetched docs, readmes, release pages, and the local codebase.
