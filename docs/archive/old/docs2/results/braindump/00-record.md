> **📦 Archived on 2026-04-23** — superseded by [Braindump Record](../../../docs/braindump/00-record.md). Kept for historical reference.

# Braindump Record

## Raw Input

> Okay, so the topic is AI Agent evals, but I feel like the problem should be generalized to just...evals.
>
> What does it mean? You have a sample dataset (or datasets) with example inputs and expected outputs (I'm not sure output is the right word here: _this_ output is _some_ reference data that the eval framework should be able to compare/reference/whatever the _actual_ output data. In other words, this is some reference based on what the real output can be _evaluated_)
>
> You send in that sample data to the...agent/actor/whatever. (Question: should there be some sort of stable identity here?)
>
> Then you _generate_ or just _get_ the real data/output. (I guess the stable identity could help here, details later)
>
> THEN you can grade/score/evaluate/check (what's the correct terminology?) the output based on the input and the reference/expected value. (This is often labeling, scoring (0-1), pass/fail, metrics (eg. latency, token count), etc)
>
> Now this scored/graded output can be used in multiple ways:
>
> - compare to earlier output(s):
>     - all outputs: historical improvements (or not). eg. you can visualize how the agent/actor/whatever changed over time
>     - previous/baseline output: you could use a main branch output as baseline and compare it to the current PR values. If it shows regression, you can either gate the PR (meaning it can't be merged) or something else.
> - absolute value checks: 1.0 is the highest, what's the real. pass or fail. segmentation, categorization, etc. Check scores against thresholds (not baselines)
>
> There are several ways to interpret and use results. The short version:
> - I have sample input and expected results
> - I feed in the sample input, check the actual result
> - Analyze/interpret the results (and act on it depending on the types of results)
>
> I've looked at a few eval frameworks: I think most of them roughly implement the above flow. Maybe not all of the features and maybe in an opinionated way. Curious to see a comparison to actual projects.
>
> ---
>
> The thing that I'm pretty sure is very different between all projects is execution: most eval tools are opinionated about the agent harness, execution platform or anything else. They are one or the other. That's something I'd like to change.
>
> For example: many eval frameworks are platforms. Many of them include an agent harness. etc
> I'd like a solution that is unopinionated about those, or at least supports a wide range of use cases.
>
> For example: I really like about agentevals (https://github.com/agentevals-dev/agentevals) that it primarily (or only) uses OTEL spans for evaluation. It doesn't actually execute anything. The user runs the agent (without code change...probably), the OTEL spans come in, then the user can compare those to eval sample data. That's quite genius actually and something I definitely want to support.
>
> I probably want to support "my own" agent harness as well. For example: I just want to run evals on a prompt, tweak it. A simple agent harness should be available.
>
> I also think making it available as a library is a must. Users should be able to integrate it into their tests/eval stuff. So a generic eval library is also part of it.
>
> I don't think platform should be a priority in the beginning, but I can see how platforms can be useful: you want to store data somewhere.
>
> To summarize the execution mode: library, binary, agent harness, pure OTEL based and anything else that makes sense. It should be low level enough to support all that.
>
> ---
>
> grading/scoring: this is another thing that solutions do differently.
>
> Many of them do "scoring" (between 0 and 1). These are either objective metrics, or a scale or something.
>
> There are also pass/fail grades.
>
> There are labels (attach labels to result or something?)
>
> Metrics: token usage, etc (when using otel this may come from there...maybe otel should be a requirement?)
>
> I'm not sure what's the right terminology: scoring, grading, whatever
>
> Also not sure what the input is.
>
> Something I saw and I can't decide: allow returning multiple scores from a scorer? or just one? are there usecases where calculating a grade takes a lot of compute, so it should be possible to compute multiple scores/grades in one?
>
> Overall, grading/scoring can be very different which is a challenge.
>
> Also, comparisons (eg. with baselines) is also a challenge due to the large number of grade outputs
>
> But, I do think most/all of them should be supported by a low level library.
>
> ---
> Generic evals: although evals are AI targeted, I don't see a reason why this can't be a generic eval library.
>
> What's this library really? You have some sample inputs, you send it into a "blackbox/function/whatever", it spits something out, you grade/score/check that output. Notice I haven't used the word AI for that description. Although it would probably be primarily used for AI evals, I think a more generic design perspective would help.
>
> ---
>
> Tech stack: I like Rust, I have an agent harness and some LLM integrations in Rust, so I think it should be Rust. Most other tools are in python I think.
>
> I like in https://github.com/agentevals-dev/agentevals that evaluators (basically another term for scorers/graders) can be written in any language: either use their python SDK or just write a binary/process and accept JSON on stdin. I think that's elegant and flexible, something like that would be nice.
>
> I don't think their evaluator catalog is important (at least not in the beginning).
>
> The challenge with rust is: the library will probably not work with other languages, so either the binary needs to be flexible enough (eg. with good configuration), or we need another way to call the rust code (FFI? should be possible with go, python, TS? (that's what restate does I think))
> So maybe Rust isn't so bad after all. And it's fast. And I like the language. (this is a good idea: base rust lib, support in many other languages, I like it)
>
> Extensibility is important: custom scorers/graders/evaluators. Custom input/output? Custom....whatever?
>
> ---
> Eventually: dashboards, CI/CD gating, etc should be available. The idea is already big. Let's build something and then expand.
>
> ---
> Storing runs? IN what format? Is there a standard?
>
> What kind of information/aggregations/etc gets stored?
> Do we store stuff when using OTEL?
>
> ---
> OTEL: I like the idea of using spans (maybe logs?) for evals. It should definitely be a run mode.
>
> Maybe use OTEL for other run modes as well?
>
> Should OTEL be a hard requirement? Probably not, but should be integrated with other stuff.
>
> https://github.com/agentevals-dev/agentevals acts as an OTEL ingest point. Is that a good idea? Should we use otel collector instead? And inject spans that way? Or query trace stores somehow?
>
> ---
> Some use cases I'd like supported:
> - I have a prompt, some samples and known good outputs. I'd like to improve the prompt.
> - I have an Excalidraw generator agent that outputs JSON (so OTEL alone may not be enough). Render it to a PNG and run evaluators on that (as well, checking the excalidraw json probably also makes sense)
> - "Check an issue on GitHub, write a failing test to confirm a bug". The output is once again not the real agent output, but running the test that fails. So eval should probably run the tests.
> - "Agent blueprint writer": Create a new agent blueprint for researching stuff. LLM returns markdown in LLM spans, eval can run on OTEL. (OTEL is powerful, where the output isn't some other artifact, OTEL could be more than enough, because it doesn't need any execution)
>
> ---
> https://github.com/agentevals-dev/agentevals
>
> I like this project, they got a lot of things well. But...python, only otel, not low level, etc.
>
> ---
>
> Questions:
> - How does an eval process look like exactly? (Like low level steps)
> - What's the different execution modes called? (eg. when just using OTEL spans or actually running within the lib/binary?)
> - Scorers? Graders? Evaluators? what's the right vocabulary?
> - Are there any features I didn't think about?

## Q&A Log

### Round 1

- **Q**: On "generic beyond AI" — Is the generic framing a design principle (keep core abstractions domain-free, then layer AI-specific stuff on top) or a market bet (you want non-AI users too)?
- **A**: Design principle.

- **Q**: On execution flexibility — You list library, binary, agent harness, and pure OTel as modes. Which would you use first and dogfood?
- **A**: Has use cases for all. Easiest to start with would be the library. Has strong OTel use cases too. Concerned that if not all modes are in the vision from the start, something might get lost in the design.

- **Q**: On the Excalidraw and GitHub-issue use cases — Is post-processing inside or outside the eval framework?
- **A**: Doesn't know. Can see either way. When using library or OTel mode, probably a post-processing step is necessary. Otherwise the agent might handle it. Needs research and weighing pros/cons.

- **Q**: On Rust + multi-language — Have you shipped FFI bindings before?
- **A**: Never did FFI before. It's just an option. Knows he wants Rust.

- **Q**: On "stable identity" — What are you imagining?
- **A**: Stable identity for the sample data: when sending data to an agent and waiting for OTel spans, how do you know which trace belongs to which sample?

### Round 2

- **Q**: On the correlation problem — inject correlation ID (requires some agent instrumentation) vs. timing heuristics (fragile). Preference? Is "zero code change" hard or soft requirement?
- **A**: Prefers zero code changes. Not sure what agentevals does for this.

- **Q**: On existing Rust work — How mature? Would eval framework build on it or be independent?
- **A**: Separate work. Wouldn't want to see AI code near the eval API. The API should be independent. Implementation can use AI stuff, but API is clean.

- **Q**: On who this is for besides you — Which user segments matter?
- **A**: Will be the first user, but expects others will use it as well.

- **Q**: On scope and ambition — Resource situation?
- **A**: Doesn't care about that concern. Can work on this.

- **Q**: On what "better" means — What would make you stop using your own tool?
- **A**: Likes high-level solutions that are "good defaults" but also wants to build from lower-level components. Flexibility is important: works on various different projects with different agents, needs flexible tools.

### Round 3

- **Q**: On the layered architecture — Can you give a concrete example of what "low-level" looks like?
- **A**: Exactly as described (scorer = function taking input+output+reference → score; high-level = pre-built scorer). But also means the APIs and how they're used. Post-processing may be available but not used in many cases. Low-level APIs should be minimal.

- **Q**: On what triggered this — Why now?
- **A**: Has been trying to use eval frameworks to improve agents, but: they want to lock him in, or no good Rust solution, not flexible enough, not well thought out enough.

- **Q**: On anti-goals — What would you consider failure?
- **A**: Confirmed: "it works but it's so complex that I dread writing new evals" and "it works but I can't extend it without modifying the core" are both anti-goals.

- **Q**: On storage and state — Querying vs reproducing?
- **A**: Storage can be delayed. If there's serializable output, the caller can handle storage. Add storage once there are enough use cases and patterns.

- **Q**: On inspirations outside eval space?
- **A**: Couldn't identify a strong external inspiration.

### Post-Round 3

- **Clarification from user**: The "simple loop" framing (input → run → output → grade) is actually two structurally different things:
  - **Integrated mode**: the framework controls execution (input → run → output → grade)
  - **Observation mode**: the framework only receives and grades (traces arrive → extract output → grade)
  - These are fundamentally different — in observation mode, the framework is a receiver and grader, not an orchestrator.

## Extracted Elements

### Ideas & Project Concepts

- **[I-01]**: A generic, domain-agnostic eval library in Rust with a layered architecture
  - Detail: Core API has no AI concepts. AI-specific scoring, agent harnesses, and LLM-as-judge live in layers above. Rust core with multi-language access via binary+JSON protocol (agentevals-dev style) and potentially FFI.
  - Specificity: concrete proposal
  - User conviction: strong opinion

- **[I-02]**: Multiple execution modes as first-class citizens
  - Detail: Library mode (in-process), binary mode (CLI), OTel observation mode (receive traces, grade without executing), agent harness mode (built-in simple agent runner). All modes should be supported by the same low-level core.
  - Specificity: concrete proposal
  - User conviction: strong opinion

- **[I-03]**: OTel-native observation mode (inspired by agentevals-dev)
  - Detail: Framework receives OTel spans from independently-running agents, extracts outputs, grades against reference data. Zero code changes on the agent side preferred.
  - Specificity: directional (correlation problem unsolved)
  - User conviction: strong opinion

- **[I-04]**: Flexible, extensible scoring/grading system
  - Detail: Support multiple score types (0-1 continuous, pass/fail, labels, metrics like latency/tokens). Custom scorers in any language via binary+JSON stdin/stdout. Open question: single vs multi-score returns per scorer.
  - Specificity: directional
  - User conviction: moderate

- **[I-05]**: Result comparison and analysis
  - Detail: Compare current run against historical runs (trend over time) or against a baseline (e.g., main branch). Absolute threshold checks. CI/CD gating based on regression detection.
  - Specificity: directional
  - User conviction: moderate

- **[I-06]**: Post-processing / transform pipeline before grading
  - Detail: For use cases where the raw output needs transformation before grading (e.g., render Excalidraw JSON to PNG, run generated tests). Placement inside vs outside framework is an open question.
  - Specificity: vague hunch
  - User conviction: moderate

- **[I-07]**: Multi-language support via Rust core
  - Detail: Rust library as the core. Access from Python/Go/TS via either FFI bindings or binary+JSON protocol. No FFI experience — binary protocol is the safer path.
  - Specificity: directional
  - User conviction: strong opinion (on Rust), moderate (on mechanism)

### Hypotheses

- **[H-01]**: Evaluation is a generic problem that doesn't require AI-specific abstractions at the core
  - Basis: The fundamental loop (input → blackbox → output → grade) is domain-agnostic. AI-specific concerns (LLM-as-judge, token metrics, trajectory matching) are layers on top, not core primitives.

- **[H-02]**: Existing eval tools are too opinionated about execution, which limits their utility
  - Basis: Direct experience trying to use eval frameworks — they lock you into their agent harness, their platform, their execution model.

- **[H-03]**: A single low-level library can support all execution modes (library, binary, OTel, agent harness)
  - Basis: If the core abstractions are right (sample data, scorer, result), the execution mode is just how the "output" gets produced — the grading step is the same regardless.

- **[H-04]**: OTel spans are sufficient as evaluation input for many (but not all) use cases
  - Basis: When the agent's output is captured in traces (text, tool calls, reasoning), OTel is enough. When the output is an artifact (PNG, test result), additional steps are needed.

- **[H-05]**: Rust is a viable and advantageous choice for an eval framework
  - Basis: Performance, language preference, existing Rust agent work. Multi-language access via binary protocol is viable (agentevals-dev proves the pattern in Python).

### Technical Opinions

- **[T-01]**: The eval API must be AI-agnostic — no AI concepts in the core API surface
  - Reasoning: Generic design principle. AI-specific layers built on top.
  - Conviction: firm conviction

- **[T-02]**: Rust for the core implementation
  - Reasoning: Performance, personal preference, existing Rust ecosystem work. "I want to use Rust, that much I know."
  - Conviction: firm conviction

- **[T-03]**: Multi-language support via binary+JSON stdin/stdout protocol (not FFI as primary path)
  - Reasoning: Never done FFI. Binary protocol is proven (agentevals-dev uses it for evaluators). Simpler to maintain.
  - Conviction: strong opinion, lightly held (FFI remains an option)

- **[T-04]**: OTel should not be a hard requirement but should be deeply integrated
  - Reasoning: Not all use cases need OTel. But when available, it's powerful. Maybe use OTel for other modes too.
  - Conviction: strong opinion, lightly held

- **[T-05]**: Storage is deferred — serializable output is sufficient initially
  - Reasoning: Let patterns emerge from use before committing to a storage model. Caller handles persistence.
  - Conviction: strong opinion

- **[T-06]**: Pre-built evaluator/scorer catalog is not important initially
  - Reasoning: Extensibility and custom scorers matter more than a catalog.
  - Conviction: moderate

### Questions the User Is Asking

- **[Q-01]**: How does an eval process look like exactly? (Low-level steps)
- **[Q-02]**: What are the different execution modes called?
- **[Q-03]**: What's the right vocabulary — scorers, graders, evaluators?
- **[Q-04]**: Are there features I didn't think about?
- **[Q-05]**: Should scorers return single or multiple scores?
- **[Q-06]**: How to correlate OTel traces to sample data without code changes on the agent?
- **[Q-07]**: What format should runs be stored in? Is there a standard?
- **[Q-08]**: Should the framework act as an OTel ingest point, use an OTel collector, or query trace stores?

### Assumptions

- **[A-01]**: The core eval loop can be cleanly separated from execution concerns
  - Source: H-03, I-01 — the idea that a single library supports all modes
  - Note: This is load-bearing. If the grading step actually needs deep knowledge of how execution happened (not just the output), this falls apart.

- **[A-02]**: Binary+JSON protocol is sufficient for multi-language support
  - Source: T-03, I-07 — the agentevals-dev pattern
  - Note: May not be sufficient for library-mode integrations where users want in-process scoring in Python/TS. Binary protocol adds process overhead.

- **[A-03]**: Zero-code-change OTel observation is achievable for trace-to-sample correlation
  - Source: I-03, Q-06 — user's preference for zero agent-side changes
  - Note: Load-bearing. If correlation requires agent instrumentation, the "pure observation" mode becomes "observation with light instrumentation."

- **[A-04]**: A Rust-based eval tool can gain adoption in a Python-dominated ecosystem
  - Source: T-02, H-05
  - Note: The research shows Python is non-negotiable for most users (Stream 6, Constraint 4). Binary+JSON protocol may bridge this, but the library-mode story for Python users needs to be strong.

- **[A-05]**: Generic (non-AI-specific) core abstractions will naturally support AI eval use cases well
  - Source: H-01, T-01
  - Note: Risk that generic abstractions miss AI-specific needs (e.g., trajectory evaluation, multi-turn conversations) that don't map cleanly to input→output→grade.

### Builder Profile

- **Domain proximity**: Practitioner — actively building AI agents in Rust, has tried multiple existing eval frameworks
- **Hands-on experience**: Used several eval frameworks (not specified which ones in detail). Found them lacking: lock-in, no Rust option, inflexible, poorly designed. Familiar with agentevals-dev specifically.
- **Unique advantages**:
  - Existing Rust agent harness and LLM integrations (separate project)
  - Comfortable building low-level systems in Rust
  - Diverse use cases (prompt tuning, Excalidraw agent, GitHub issue agent, blueprint writer) — this diversity drives the flexibility requirement
  - Direct practitioner pain — building for himself first
- **What triggered this exploration**: Ongoing frustration with existing eval tools while trying to improve agents. No single trigger — accumulated pain from lock-in, inflexibility, and absence of good Rust tooling.

### Anti-Goals & Failure Definitions

- Tool becomes so complex that writing new evals feels like a chore
- Can't extend the tool without modifying the core
- AI concepts leak into the core API (violating the generic design principle)
- Lock-in to a specific execution mode, agent framework, or platform
- Building a platform before the core is solid
- Premature storage/persistence model that constrains future design

### Resource Constraints

- **Capacity**: Solo (implied — "I can work on this")
- **Time**: Not concerned — willing to invest
- **Funding**: Not discussed, not a concern
- **Distribution**: Cold start — no established audience for this tool specifically, but has existing Rust AI work
- **Timeline**: Casual exploration — no external deadline or window pressure

### Evaluation Lens

- **How the user approaches problems**: Composability-oriented, layered thinking. Wants to understand the low-level primitives before building high-level abstractions. Values flexibility over convenience.
- **What "better" means**: More flexible, less opinionated, properly layered (high-level defaults + low-level components), works across diverse use cases without modification.
- **Composability preference**: Strong. Thinks in layers. Wants to assemble solutions from components, not adopt monoliths.

### Inspirations & Analogies

- **agentevals-dev**: Primary inspiration. Likes: OTel-native evaluation, evaluators in any language via binary+JSON stdin/stdout, no execution — just observation. Dislikes: Python-only, only OTel, not low-level enough.
- No strong inspirations from outside the eval domain identified.

### Scope Boundaries

- **Not interested in**: Building a platform first. Pre-built evaluator catalogs (initially). Storage as a first-class concern (deferred). Competing on metrics.
- **Target audience**: Himself first. Developers building diverse AI agents who need flexible, non-opinionated eval tooling.
- **Explicitly deferred**: Dashboards, CI/CD gating, storage — "eventually" but not now.
