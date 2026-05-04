# Acquisition Naming Discussion

**Date:** 2026-04-26
**Topic:** Whether the term "acquisition" fits the eval-kernel domain, and what to use instead.

---

## User

> What do you think? The acquisition domain word: does it fit into the domain? Research the domain, analyze what acquisition is in this project. Give me options for an alternate, better word.

## Assistant

**Verdict**

Acquisition is technically accurate but not a good domain word. In this project it means: "get the candidate output/evidence for a sample before scoring it."

It appears as the core seam in `src/acquisition.rs`, is required by `Run::builder().acquisition(...)`, powers `Eval::new(...).acquire(...)`, backs HTTP/subprocess plugins, and also covers Observe, which reads OTel spans instead of calling the system.

The problem: AI eval domain language does not use "acquisition" for this. Competitors tend to say task, solver, model/provider, or just treat it as the user callback. Your own archived glossary says Acquisition has "No domain equivalent" and is a novel term. It also collides with M&A language already present in `docs/competitive-analysis-2026-04.md`.

**Best Options**

| Option | Fit | Notes |
|---|---|---|
| Task | Best public API | Matches Braintrust/evalite data + task + scorers. Great for inline evals. Weak for observe mode because observe does not run the task. |
| OutputSource | Best single replacement | Accurately covers inline, HTTP, subprocess, and trace observe. Less domain-native, but clearer than Acquisition. |
| CandidateSource | Strong eval semantics | Makes clear this produces the candidate output being scored. Risk: candidate already means candidate run in comparisons. |
| Target / TargetSystem | Good mental model | Names the system under evaluation. Risk: "target" can mean expected output in eval tooling and already appears internally as scoring target. |
| Invocation | Good for inline only | Clear when calling an app/model. Bad fit for observe/trace mode. |
| OutputProvider | Clear but risky | Descriptive, but "provider" already means model/provider integration in this repo. |

**Recommendation**

Use Task for the happy-path/public facade, and OutputSource for the low-level trait if you keep one abstraction spanning inline execution and trace observation.

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

---

## User

> OutputSource is a bit weird when there is a task to be run. Task is weird when there is no task, but some source to load data from.
> Acquisition is nice from this perspective, but sounds weird and out of domain.

## Assistant

Exactly. The abstraction is real; the word is the problem.

I'd avoid forcing either Task or OutputSource as the one canonical term. They each describe only one mode well.

Better framing: this abstraction is not "the task" and not "the source"; it is the thing that produces evaluation evidence for a sample.

Best candidates:

| Word | Why It Fits | Weakness |
|---|---|---|
| Evidence / EvidenceSource | Covers model output, traces, trajectories, spans, retrieved prior results. Scorers consume evidence. Strong for both active and passive modes. | Slightly legal/forensic, less common in eval APIs. |
| Observation / ObservationSource | Covers "we observed the system's behavior," whether by running it or reading traces. Domain-adjacent via observability. | Collides somewhat with existing observe mode. |
| Response / ResponseSource | Natural for LLM/app evals: input goes in, response comes out. | Too narrow for traces, trajectories, tool calls, metrics. |
| Subject / EvalSubject | Names the system under evaluation rather than the operation. Works whether it is callable or observed. | Abstract; method naming still needs care. |
| Elicitation | More domain-relevant than acquisition: prompt/input elicits behavior. Covers active runs well. | Bad for passive trace loading. |
| Collection / Collector | Better than acquisition for "gather outputs/traces." | Sounds operational, not eval-domain; weak for inline task execution. |

**My recommendation: EvidenceSource.**

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

Why I like it: scorers do not fundamentally require an "output." They require something to evaluate. Sometimes that is a string answer. Sometimes it is spans. Sometimes it is a trajectory. "Evidence" is the clean umbrella.

---

## User

> let's try a few more

## Assistant

More options, grouped by what they emphasize:

| Term | Best For | Problem |
|---|---|---|
| Evidence | Broadest semantic fit: output, trace, trajectory, transcript, spans | Slightly heavy/forensic |
| Artifact | "Thing produced or collected for scoring" | Generic, can mean files/build outputs |
| SubjectOutput | Clear that this is from the system under evaluation | Awkward as trait/API name |
| Observation | Works for active and passive evals: observe behavior | Collides with observe mode, passive-sounding |
| Capture | "Capture the system behavior for scoring" | Verb/noun ambiguity; operational |
| Response | Best for LLM/product outputs | Too narrow for traces/spans |
| Behavior | Agent/domain-strong: evaluate behavior, not just answers | Abstract; BehaviorSource sounds odd |
| ResultSource | Produces the result to score | Collides with RunResult, SampleResult |
| TrialSource | Produces one trial's material | Too coupled to multi-trial execution |
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

**My short list after this pass:**
1. EvidenceSource
2. ObservationSource
3. BehaviorSource
4. ResponseSource
5. Resolver

If the API needs to feel clean in Rust, I'd test these signatures:

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

---

## User

> EvidenceSource vs Acquisition?

## Assistant

I'd choose EvidenceSource over Acquisition.

| Dimension | Acquisition | EvidenceSource |
|---|---|---|
| Domain fit | Weak. Not used in eval tooling. | Better. Evals score evidence of behavior. |
| Active task fit | Good: "acquire output by running something." | Okay: task produces evidence. |
| Passive trace fit | Good: "acquire spans from backend." | Good: trace backend supplies evidence. |
| First-read clarity | Low. Needs glossary. | Medium-high. Broad but understandable. |
| API feel | acquisition(...), AcquisitionError sound odd. | evidence_source(...), EvidenceError sound purposeful. |
| Risk | Sounds like M&A/data procurement. | Sounds slightly forensic/formal. |

The key difference:
- **Acquisition** names the operation: obtaining something.
- **EvidenceSource** names the role in evaluation: supplying what scorers inspect.

Since this project is an eval kernel, I'd prefer the eval-role word.

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

If EvidenceSource feels too long for the builder, use `evidence(...)` publicly and keep the trait explicit:
```rust
Run::builder()
    .dataset(dataset)
    .evidence(task)
```

**Final take:** Acquisition is architecturally elegant but product-weird. EvidenceSource is less elegant, but users will understand it faster and it fits both "run task" and "load observed behavior" better than Task or OutputSource.

---

## Assistant (offered next steps)

It looks like the transcript ends on my prior recommendation (EvidenceSource over Acquisition) without a new question from you. Want me to:

1. **Sketch the rename** — concrete diff plan: trait, methods, builder, error types, plugin protocol field names, doc/glossary updates.
2. **Pressure-test EvidenceSource** — find where it breaks down in the existing code (e.g., `acquire_with_snapshots`, `AcquisitionMetadata`, plugin protocol JSON) before committing.
3. **Explore a different axis** — e.g., split the abstraction into two named things (active `Task` + passive `Trace`/`Observation`) instead of one umbrella, since the awkwardness mostly comes from forcing one word over both modes.
4. **Stay with Acquisition** — and just sharpen the docs/glossary so the term reads less like M&A.

Which direction?

---

## User

> 2

## Assistant

Pressure test of EvidenceSource against the actual code surface:

### Where it holds up

| Touchpoint | Rename | Reads as |
|---|---|---|
| `AcquisitionError` variants | `EvidenceError::TraceNotFound`, `::BackendUnavailable`, `::Timeout` | Clean. "No evidence for sample X" is natural. |
| `Observe` impl (`evalkit-otel/src/lib.rs:144`) | `impl EvidenceSource<I, Vec<Span>> for Observe` | **Better** than `Acquisition`. Observe is genuinely a source, not an acquirer. |
| `AcquisitionSnapshot<O>` | `EvidenceSnapshot<O>` | Fine. |
| `metadata().mode("observe")` (`evalkit-otel/src/lib.rs:163`) | unchanged values, just `EvidenceMetadata` wrapper | Fine. |

### Where it breaks down

**1. The trait method name (`src/acquisition.rs:118`)**
```rust
async fn acquire(&self, input: &I) -> Result<O, AcquisitionError>;
```
becomes
```rust
async fn evidence_for(&self, input: &I) -> Result<O, EvidenceError>;
```
`acquire` is a verb. `evidence` is a noun. There is no good verb form. `evidence_for`, `produce_evidence`, `gather` all read worse than `acquire`. Every implementor and every call site eats this regression — and the inline closure case is the dominant usage.

**2. `AcquiredOutput<O>` (`src/acquisition.rs:50`)**
```rust
pub struct AcquiredOutput<O> {
    pub output: O,
    pub snapshots: Vec<AcquisitionSnapshot<O>>,
}
```
The semantic load of "acquired" is *we finished acquiring* — that's why `output` and `snapshots` co-exist (final + intermediates). `Evidence<O>` flattens that distinction: snapshots are evidence, output is evidence, what's the difference? You'd end up with `EvidenceWithSnapshots<O>` or similar, which is uglier than today.

**3. `acquire_with_snapshots` (`evalkit-runtime/src/lib.rs:98, 1433`)**
The Scrubber trait, partial scoring plans, and the runtime executor all hinge on this method. It describes capturing mid-flight artifacts during execution. `evidence_with_snapshots` is technically fine but loses the "we're doing work and emitting intermediates" framing.

**4. Plugin protocol — wire format (`docs/plugin-protocol.md`)**
```json
{ "kind": "acquisition", "name": "demo-plugin", ... }
```
```toml
[acquisition]
command = ["python3", "model.py"]
```
- `kind` is a discriminated union: `"acquisition" | "scorer"`. Both are **steps in a pipeline**. Renaming to `"evidence_source"` makes one a role-noun and the other still a step-noun — asymmetric. (And `"scorer"` is already perfect, so you can't fix it by renaming both.)
- TOML key `[acquisition]` → `[evidence_source]` breaks every user config and every plugin.
- This is the most expensive churn for the least gain: plugin authors don't need a domain word, they need a stable ID.

**5. JSONL run-log v1 + SQLite (`docs/schema/run-log-v1.schema.json:426`, `evalkit-server/src/lib.rs:651`)**
```json
"acquisition_mode": { "type": "string", "minLength": 1 }
```
```sql
acquisition_mode TEXT NOT NULL,
```
Rename → schema v2 bump or dual-read shim. Stored runs and exports become incompatible. *This field is not really about the trait — it labels how the output was obtained ("inline" vs "observe"). `evidence_mode` is worse than `acquisition_mode` at expressing that.*

**6. Public facade (`src/eval.rs`)**
```rust
Eval::new(samples).acquire(|input| async { ... }).scorer(...).run()
```
becomes
```rust
Eval::new(samples).evidence(|input| async { ... }).scorer(...).run()
// or .evidence_source(...) — too long for the happy-path facade
```
For the 80% inline-closure case, "this is my evidence source" misframes what the user is doing (calling their model). `acquire` is novel but accurate.

**7. `Run::builder().acquisition(observe)` (`src/run.rs`)**
Less bad. `.evidence_source(observe)` works because it's the observe mode where the framing actually fits.

### Tally

- **Files touched on rename:** ~41 non-archive + 3 wire formats (JSONL schema v1, SQLite column, plugin protocol).
- **Reads better with EvidenceSource:** `Observe` impl, the observe-mode mental model, error variants in trace-loading.
- **Reads worse with EvidenceSource:** every trait method (`acquire` → no good verb), `AcquiredOutput`, `acquire_with_snapshots`, plugin protocol `kind`, the `acquisition_mode` label, the inline-closure facade.

### What this exposes

The real problem isn't the word — it's that **one trait covers two genuinely different things**:
- **Active execution** ("run my agent on this input") — Acquisition fits, EvidenceSource doesn't really
- **Passive trace loading** ("fetch what already happened") — EvidenceSource fits, Acquisition doesn't really

Forcing one word to cover both is what makes both candidates feel awkward in the wrong half of the API.

### Recommendation

**Don't switch to EvidenceSource.** It loses on more axes than it wins, and the wins concentrate in the observe path — which is also what option 3 (split the abstraction) would address more cleanly.

Two viable next moves:
- **Keep Acquisition, sharpen the glossary.** Cheapest. Add a one-paragraph "what acquisition means here" to docs and stop apologizing for it. The M&A vibe lives in conversation, not in code.
- **Split into `Task` (active) + `TraceSource` / `Observation` (passive)** with a small unifying trait the executor uses internally. Bigger refactor, but resolves the real tension instead of relabeling it.

Want me to sketch the split?
