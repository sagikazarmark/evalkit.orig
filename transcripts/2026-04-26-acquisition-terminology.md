# Conversation Transcript: Acquisition Terminology

Date: 2026-04-26

## User

What do you think? The acquisition domain word: does it fit into the domain? Research the domain, analyze what acquisition is in this project. Give me options for an alternate, better word.

Thinking: Analyzing acquisition terminology
I need to address the user's question about the term "acquisition" and whether it fits within the domain of their project. This involves researching and analyzing the domain to propose better alternatives for the term. I’m considering using tools for an in-depth examination, like inspecting the codebase and finding instances of "acquisition." This seems multi-layered, likely needing around three distinct steps. I’ll keep this detailed yet avoid complexity unless necessary.

Thinking: Inspecting project structure
I think I can directly use Glob and Grep to investigate the project structure. First, I’ll comment on my progress by saying, "I'll inspect where 'acquisition' appears and consider the surrounding language." Then, I plan to run Grep for instances of "acquisition" and review key files, such as README or package files. Avoiding Bash for file operations should keep this simpler. I’ll utilize multi_tool_use.parallel for efficiency while ensuring to use commentary only throughout the process.

I’ll inspect where acquisition appears and the surrounding product/domain vocabulary first, then map what the concept actually represents before suggesting replacement terms.

✱ Grep "acquisition|Acquisition|ACQUISITION" in . (1119 matches)
✱ Glob "**/{README,README.md,ARCHITECTURE.md,CONTRIBUTING.md,AGENTS.md,package.json,pyproject.toml,Cargo.toml,go.mod}" in . (16 matches)
✱ Glob "**/*.{md,mdx,txt}" in . (100 matches)

acquisition is heavily used in core APIs, tests, plugin protocols, and competitive docs, so I’m narrowing to the canonical definitions and user-facing docs rather than counting every occurrence.

→ Read . [offset=1, limit=200]
→ Read Cargo.toml [offset=1, limit=220]
→ Read src/lib.rs [offset=1, limit=240]
→ Read src/acquisition.rs [offset=1, limit=280]
→ Read src/eval.rs [offset=1, limit=280]
→ Read src/run.rs [offset=1, limit=340]
✱ Grep "acquire_with_snapshots|AcquiredOutput|AcquisitionSnapshot|snapshots|acquisition_mode|observe|TraceNotFound|BackendUnavailable" in src (43 matches)
✱ Grep "Acquisition|acquisition|acquire|model|provider|target|task|answer|output|completion" in docs (2273 matches)
✱ Grep "Acquisition|acquisition|acquire" in evalkit-providers (68 matches)
✱ Grep "Acquisition|acquisition|acquire" in evalkit-scorers-llm
→ Read docs/eval-facade.md [offset=1, limit=240]
→ Read docs/plugin-protocol.md [offset=1, limit=260]
→ Read evalkit-providers/src/lib.rs [offset=1, limit=260]
→ Read evalkit-providers/src/lib.rs [offset=260, limit=180]
→ Read evalkit-cli/src/main.rs [offset=1, limit=320]
✱ Glob "examples/**/*" in . (3 matches)
✱ Grep "\.acquire\(|\.acquisition\(|acquisition\]" in . (41 matches)
✱ Grep "\[acquisition\]|acquisition\.|SubprocessAcquisition|HttpAcquisition" in .
→ Read examples/quickstart.rs [offset=1, limit=180]
→ Read examples/basic.rs [offset=1, limit=180]
→ Read docs/cli-config.md [offset=1, limit=260]
→ Read docs/root-crate-boundary-audit.md [offset=1, limit=220]
→ Read docs/evalkit-kernel-boundary-plan.md [offset=1, limit=260]
→ Read evalkit-otel/tests/observe.rs [offset=1, limit=280]
→ Read evalkit-otel/src/lib.rs [offset=1, limit=340]
✱ Grep "AcquisitionMetadata|metadata\(|mode\(\"observe\"|impl Acquisition" in evalkit-otel (5 matches)
✱ Grep "task|Task|target|Target|system under test|SUT|candidate|generate|Generation|Inference|Invocation|Execution|Call" in docs/archive/old/docs2/results/research (88 matches)
→ Read docs/archive/old/docs2/results/research/02-domain-language.md [offset=1, limit=260]
→ Read docs/archive/old/docs2/results/spec/domain-language-review.md [offset=1, limit=320]
→ Read docs/competitive-analysis-2026-04.md [offset=430, limit=130]

Thinking: Investigating research needs
I’m considering if there are non-archived documents or research available. It seems the glob tool isn’t showing certain documents, possibly because they’re archived. There’s a mention of some being superseded, but I don't see those in the current glob. I need to figure out whether the term “acquisition” fits within the domain. The user wants me to research this domain, which might involve both internal and external sources. Maybe I should include some competitive documents or check current external resources.

Thinking: Analyzing documentation needs
I see we have repository documents with various sources, and there's a thought about using webfetch for external documents if that helps. However, we might have enough already. I should consider fetching Braintrust and inspect documents since the tool is available. If webfetch fails, we still have competitive documents to reference. I want to verify terms using known URLs while making my answer concise and reference-rich. I also need to analyze “acquisition” in comparison to alternatives and look into specific lines of code for further insights.

→ Read src/run.rs [offset=540, limit=70]
→ Read src/run.rs [offset=900, limit=60]
→ Read src/run_result.rs [offset=1, limit=120]
→ Read evalkit-cli/src/main.rs [offset=340, limit=240]
→ Read docs/archive/old/docs2/results/research/02-domain-language.md [offset=330, limit=140]
→ Read docs/archive/old/docs2/results/research/04-architecture.md [offset=100, limit=80]
→ Read docs/archive/old/docs2/results/spec/eval-kernel.md [offset=1, limit=260]
✱ Grep "Acquisition|acquisition|acquire|Output|output|task|target|model" in docs/archive/old/docs2/results/spec/eval-kernel.md (93 matches)
→ Read TODOS.md [offset=1, limit=240]
→ Read docs/ROADMAP.md [offset=1, limit=260]

Thinking: Locating documentation
I need to make sure I find the current documents or specifications, especially since it mentions that it's superseded. There's a chance that the path is broken due to the archive situation. I should probably use the global documents or specifications to see if I can find what I'm looking for. It's really important to get to the final recommendation without any confusion!

✱ Glob "docs/spec/**/*.md" in .
✱ Glob "docs/research/**/*.md" in .
✱ Grep "Acquisition \| How outputs|How outputs are obtained|The architectural boundary|No domain equivalent" in docs (14 matches)

Thinking: Considering naming options
I think I should read through the decisions log to explore possible naming options. It feels like a good approach to gather insights from previous decisions. There might be useful information there that could guide my choice and help me find the most fitting name. I can also jot down any interesting ideas that come up while I review. Let’s see what I can uncover in the log!

→ Read docs/archive/old/docs2/results/spec/decisions-log.md [offset=1, limit=90]
→ Read docs/archive/old/docs2/results/spec/glossary-rewrite.md [offset=1, limit=80]

Thinking: Clarifying terms in async tasks
I’m thinking there’s a potential confusion with async tasks. For example, the term Generator in LLM output generation could be mixed up with how it pertains to spans returned by Observe. Also, I'm unsure if agents, retrievers, and tool flows should just be classified as generation. Then there's Producer or OutputProvider, which is generic but already feels heavy as "provider" is commonly used for model/client contexts. It’s tricky!

✱ Grep "candidate output|Candidate output|actual output|actual_output|output under test|system under evaluation|system under test|SUT|target agent|target system" in docs (29 matches)
✱ Grep "Candidate|candidate|baseline" in src (94 matches)
✱ Grep "ScoringTarget|Target|target" in src (63 matches)

Verdict
Acquisition is technically accurate but not a good domain word. In this project it means: “get the candidate output/evidence for a sample before scoring it.”
It appears as the core seam in src/acquisition.rs, is required by Run::builder().acquisition(...), powers Eval::new(...).acquire(...), backs HTTP/subprocess plugins, and also covers Observe, which reads OTel spans instead of calling the system.
The problem: AI eval domain language does not use “acquisition” for this. Competitors tend to say task, solver, model/provider, or just treat it as the user callback. Your own archived glossary says Acquisition has “No domain equivalent” and is a novel term. It also collides with M&A language already present in docs/competitive-analysis-2026-04.md.

Best Options

| Option | Fit | Notes |
|---|---|---|
| Task | Best public API | Matches Braintrust/evalite data + task + scorers. Great for inline evals. Weak for observe mode because observe does not run the task. |
| OutputSource | Best single replacement | Accurately covers inline, HTTP, subprocess, and trace observe. Less domain-native, but clearer than Acquisition. |
| CandidateSource | Strong eval semantics | Makes clear this produces the candidate output being scored. Risk: candidate already means candidate run in comparisons. |
| Target / TargetSystem | Good mental model | Names the system under evaluation. Risk: “target” can mean expected output in eval tooling and already appears internally as scoring target. |
| Invocation | Good for inline only | Clear when calling an app/model. Bad fit for observe/trace mode. |
| OutputProvider | Clear but risky | Descriptive, but “provider” already means model/provider integration in this repo. |

Recommendation
Use Task for the happy-path/public facade, and OutputSource for the low-level trait if you keep one abstraction spanning inline execution and trace observation.

Example direction:

```rust
Eval::new(samples)
    .task(task)
    .scorer(scorer)
    .run()
    .await?;
```

Low-level:

```rust
trait OutputSource<I, O> {
    async fn output_for(&self, input: &I) -> Result<O, OutputSourceError>;
}
```

If you want one word everywhere, pick OutputSource. If you want the best developer-facing domain language, pick Task and let observe mode be its own explicit path.

▣  Build · GPT-5.5 · 1m 47s

OutputSource is a bit weird when there is a task to be run. Task is weird when there is no task, but some source to load data from.
Acquisition is nice from this perspective, but sounds weird and out of domain.

Thinking: Evaluating options for response
I think the user wants further options for naming conventions, perhaps focusing on a two-axis naming pattern with terms like "Target," "System," or "Subject." I should aim for clarity, balancing active task execution and passive retrieval of data.
Some possible suggestions could be "Elicit," "Collect," "Capture," "Produce," "Resolve," "Output," "Observation," "RunSource," "EvidenceSource," or "Stimulus." I need to consider these thoroughly for the best fit. It might help to distinguish the conceptual layers involved as well.

Thinking: Analyzing naming conventions
I see some potential conflicts with the term "observe mode." "EvidenceSource" is useful for both traces and task outputs, but it seems like scoring should focus on "candidate output" rather than just evidence.
I’m considering names like "ResponseSource," but that might not always fit since not all outputs are direct responses. Terms like "Subject" or "SystemUnderTest" might clarify the role better, but "OutputFactory" feels off, and "Producer" is too generic.
For user-facing names, I might suggest using "Task" for inline functions, and "ObservationSource" for observation actions. I also think we should keep internal naming straightforward, possibly with low-level concepts like Subject<I,O> that could implement various sources.

Thinking: Exploring evaluation domain terms
I'm reflecting on the key components relevant to AI and LLM evaluation. It seems important to analyze terms like Dataset, Sample, Input, Reference, Output, Scorer, Score, Run, and Trial. Each term carries significance that helps structure how evaluations are conducted. They define the processes and metrics involved in assessing AI systems. I want to ensure that I have a comprehensive understanding of these elements as they will inform both my framework and evaluation strategies effectively.

Exactly. The abstraction is real; the word is the problem.
I’d avoid forcing either Task or OutputSource as the one canonical term. They each describe only one mode well.
Better framing: this abstraction is not “the task” and not “the source”; it is the thing that produces evaluation evidence for a sample.

Best candidates:

| Word | Why It Fits | Weakness |
|---|---|---|
| Evidence / EvidenceSource | Covers model output, traces, trajectories, spans, retrieved prior results. Scorers consume evidence. Strong for both active and passive modes. | Slightly legal/forensic, less common in eval APIs. |
| Observation / ObservationSource | Covers “we observed the system’s behavior,” whether by running it or reading traces. Domain-adjacent via observability. | Collides somewhat with existing observe mode. |
| Response / ResponseSource | Natural for LLM/app evals: input goes in, response comes out. | Too narrow for traces, trajectories, tool calls, metrics. |
| Subject / EvalSubject | Names the system under evaluation rather than the operation. Works whether it is callable or observed. | Abstract; method naming still needs care. |
| Elicitation | More domain-relevant than acquisition: prompt/input elicits behavior. Covers active runs well. | Bad for passive trace loading. |
| Collection / Collector | Better than acquisition for “gather outputs/traces.” | Sounds operational, not eval-domain; weak for inline task execution. |

My recommendation: EvidenceSource.

It resolves your exact tension:

```rust
Run::builder()
    .dataset(dataset)
    .evidence_source(source)
    .scorer(scorer)
```

Active implementation:

```rust
TaskEvidence::from_fn(|input| async move {
    call_model(input).await
})
```

Passive implementation:

```rust
TraceEvidence::from_backend(jaeger)
```

Trait shape:

```rust
trait EvidenceSource<I, E> {
    async fn evidence_for(&self, input: &I) -> Result<E, EvidenceError>;
}
```

Why I like it: scorers do not fundamentally require an “output.” They require something to evaluate. Sometimes that is a string answer. Sometimes it is spans. Sometimes it is a trajectory. “Evidence” is the clean umbrella.

▣  Build · GPT-5.5 · 17.6s

let's try a few more

More options, grouped by what they emphasize:

| Term | Best For | Problem |
|---|---|---|
| Evidence | Broadest semantic fit: output, trace, trajectory, transcript, spans | Slightly heavy/forensic |
| Artifact | “Thing produced or collected for scoring” | Generic, can mean files/build outputs |
| SubjectOutput | Clear that this is from the system under evaluation | Awkward as trait/API name |
| Observation | Works for active and passive evals: observe behavior | Collides with observe mode, passive-sounding |
| Capture | “Capture the system behavior for scoring” | Verb/noun ambiguity; operational |
| Response | Best for LLM/product outputs | Too narrow for traces/spans |
| Behavior | Agent/domain-strong: evaluate behavior, not just answers | Abstract; BehaviorSource sounds odd |
| ResultSource | Produces the result to score | Collides with RunResult, SampleResult |
| TrialSource | Produces one trial’s material | Too coupled to multi-trial execution |
| Attempt | Strong for stochastic evals: each sample gets attempts | Better names trial than output acquisition |
| Execution | Active task runs | Bad for passive trace replay |
| RunStep | Fits pipeline stage | Too generic/internal |
| Producer | Rusty/generic: produces O from I | Not domain language |
| Generator | LLM-friendly: generate candidate output | Too model/text-specific |
| Responder | Nice for QA/chat systems | Too conversational |
| Emitter | Produces observable output/spans | More telemetry than eval |
| Deriver | Derives evaluable material from input/context | Weird |
| Resolver | Resolves a sample into something scorable | Common software term, less eval-native |
| Materializer | Materializes evidence/output for scoring | Heavy, infrastructure-flavored |
| Specimen | Thing being evaluated | Too biological/weird |

My short list after this pass:

1. EvidenceSource
2. ObservationSource
3. BehaviorSource
4. ResponseSource
5. Resolver

If the API needs to feel clean in Rust, I’d test these signatures:

```rust
trait EvidenceSource<I, O> {
    async fn evidence(&self, input: &I) -> Result<O, EvidenceError>;
}

trait ObservationSource<I, O> {
    async fn observe(&self, input: &I) -> Result<O, ObservationError>;
}

trait Resolver<I, O> {
    async fn resolve(&self, input: &I) -> Result<O, ResolveError>;
}
```

Resolver is less domain-pure but has the best verb ergonomics: sample input gets resolved into the thing scorers evaluate.

▣  Build · GPT-5.5 · 9.8s

EvidenceSource vs Acquisition?

Thinking: Evaluating evidence terminology
I’m considering the term "source" and its loading vibe. It strikes me as somewhat passive. A task can still serve as an evidence source, which feels a bit strange. Perhaps I could use something like trait EvidenceProvider or EvidenceFn instead? "EvidenceSource" seems less weird and makes sense for outsiders. I think there's a fine line between how I should name it and how users might interpret it without needing a glossary.

Thinking: Proposing naming conventions
I’m thinking about the naming conventions here. If we're dealing with a kernel-internal primitive that spans both active and passive, then EvidenceSource seems like the better choice. However, if it’s more about architecture notes or internal code, Acquisition might work fine. But for a public fluent API, maybe we should steer clear of both terms. Something like Eval::new(samples).task(...) for high-level use or Run::builder().evidence(...) for low-level seems worthwhile to consider.

I’d choose EvidenceSource over Acquisition.

| Dimension | Acquisition | EvidenceSource |
|---|---|---|
| Domain fit | Weak. Not used in eval tooling. | Better. Evals score evidence of behavior. |
| Active task fit | Good: “acquire output by running something.” | Okay: task produces evidence. |
| Passive trace fit | Good: “acquire spans from backend.” | Good: trace backend supplies evidence. |
| First-read clarity | Low. Needs glossary. | Medium-high. Broad but understandable. |
| API feel | acquisition(...), AcquisitionError sound odd. | evidence_source(...), EvidenceError sound purposeful. |
| Risk | Sounds like M&A/data procurement. | Sounds slightly forensic/formal. |

The key difference:

Acquisition names the operation: obtaining something.
EvidenceSource names the role in evaluation: supplying what scorers inspect.

Since this project is an eval kernel, I’d prefer the eval-role word.

Best shape:

```rust
trait EvidenceSource<I, E> {
    async fn evidence_for(&self, input: &I) -> Result<E, EvidenceError>;
}
```

Then implementations can be active or passive:

```rust
Run::builder()
    .dataset(dataset)
    .evidence_source(task)
    .scorer(scorer)

Run::builder()
    .dataset(dataset)
    .evidence_source(trace_source)
    .map_output(extract_answer)
    .scorer(scorer)
```

If EvidenceSource feels too long for the builder, use evidence(...) publicly and keep the trait explicit:

```rust
Run::builder()
    .dataset(dataset)
    .evidence(task)
```

Final take: Acquisition is architecturally elegant but product-weird. EvidenceSource is less elegant, but users will understand it faster and it fits both “run task” and “load observed behavior” better than Task or OutputSource.

## Assistant

I’ll ground this in the current code/docs rather than just the pasted thread, focusing on what the abstraction does and how it appears in public-facing APIs.

The code confirms the abstraction has to cover three shapes: running inline work, calling external provider/plugin work, and reading observed trace-backed behavior. That makes “one perfect noun” unlikely; the tradeoff is between domain clarity and mode coverage.

I’d pick `EvidenceSource` over `Acquisition`, but I would not expose `evidence_source(...)` everywhere.

`Acquisition` is elegant internally because it means “obtain the thing to score,” whether by running a task, calling HTTP, invoking a plugin, loading fixtures, or reading traces. But it is not eval-domain language. Users will ask “acquisition of what?” and it carries M&A/customer-acquisition baggage.

`EvidenceSource` is better because it names the eval role: it supplies the material scorers inspect. That material can be an answer, trace, trajectory, snapshot stream, or prior output.

Best split:

```rust
trait EvidenceSource<I, E> {
    async fn evidence_for(&self, input: &I) -> Result<E, EvidenceError>;
}
```

Public happy path:

```rust
Eval::new(samples)
    .task(task)
    .scorer(scorer)
    .run()
    .await?;
```

Lower-level / passive path:

```rust
Run::builder()
    .dataset(dataset)
    .evidence(observe)
    .scorer(trace_scorer)
```

So my recommendation:

- Rename the umbrella trait from `Acquisition` to `EvidenceSource`.
- Rename the builder method to `.evidence(...)`, not `.evidence_source(...)`.
- Add `.task(...)` on the facade for the common active case.
- Keep `Observe` / trace APIs explicit for passive cases.

If forced to choose one visible word everywhere: `Evidence` / `EvidenceSource` beats `Acquisition`. If allowed a layered API: `task` for active user code, `evidence` for the kernel abstraction.

## User

Pressure-test EvidenceSource

## Assistant

`EvidenceSource` survives the core abstraction test, but it has real API smell in simple cases.

**Where It Wins**
- Covers active and passive modes: task call, HTTP plugin, cached fixture, trace lookup.
- Covers future richer outputs: answer strings, spans, trajectories, transcripts, intermediate snapshots.
- Names the role relative to scoring: scorers inspect evidence.
- Avoids the out-of-domain feel of `Acquisition`.
- Makes failures clearer than `AcquisitionError`: “failed to collect evidence” is easier than “acquisition failed.”

**Where It Strains**
- For basic evals, `evidence` is too formal:

```rust
Eval::new(samples)
    .evidence(task)
    .scorer(exact_match)
```

This feels worse than:

```rust
Eval::new(samples)
    .task(task)
    .scorer(exact_match)
```

- `EvidenceSource` still has “source” awkwardness when the thing is a function you run, not a store you read from.
- `ctx.evidence` is probably worse than `ctx.output` for common scorers like exact match, contains, JSON validity, etc.
- `[evidence]` in CLI config sounds less obvious than `[task]`, `[target]`, or `[runner]`.
- Plugin handshake `kind = "evidence"` is semantically accurate but a little weird.
- `EvidenceError` is broad. It can hide whether the failure was execution, backend lookup, timeout, or protocol.

**Compared To Acquisition**
`Acquisition` is better as an implementation concept. It precisely names the operation: obtain the output/material.

`EvidenceSource` is better as product/API language. It names why the thing exists: to give scorers something to evaluate.

So the decision is really:

```text
Acquisition = operation-oriented, internal elegance
EvidenceSource = eval-oriented, user comprehension
```

**Hardest Counterexample**
Simple LLM eval:

```rust
let result = Eval::new(samples)
    .evidence_source(|input| async move {
        Ok(call_model(input).await?)
    })
    .scorer(ExactMatch)
    .run()
    .await?;
```

That is too abstract for hello-world. Users are thinking “task,” not “evidence source.”

**Best Compromise**
Use `EvidenceSource` only below the facade.

Public facade:

```rust
Eval::new(samples)
    .task(task)
    .scorer(scorer)
```

Kernel/lower-level:

```rust
Run::builder()
    .dataset(dataset)
    .evidence(source)
    .scorer(scorer)
```

Types:

```rust
trait EvidenceSource<I, O> {
    async fn produce(&self, input: &I) -> Result<O, EvidenceError>;
}
```

But keep scorer language as `output`, not `evidence`:

```rust
ctx.output
ctx.reference
```

**Verdict**
`EvidenceSource` is better than `Acquisition` for the umbrella trait, but only if you do not make users say it in the happy path.

If the API must expose one word everywhere, I’d still pick `Acquisition` over forcing `evidence` into simple task evals. If you can layer the API, the best design is:

- Happy path: `task`
- Kernel abstraction: `EvidenceSource`
- Scorer context: `output`
- Trace mode: `Observe`
- Config/plugin vocabulary: maybe `target` or `source`, not necessarily `evidence` everywhere.

## User

Write the entire conversation in to transcripts/ directory
