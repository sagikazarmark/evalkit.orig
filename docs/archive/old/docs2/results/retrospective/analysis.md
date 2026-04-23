> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Pipeline Retrospective — Full Analysis

## Meta
- **Project**: evalkit (Eval Kernel direction)
- **Date**: 2026-04-04
- **Pipeline version**: research-v2.md, braindump.md, brainstorm.md, specify.md, spec-domain-review.md, implement.md, conformance (prompt not available)
- **Steps executed**: Research, Braindump, Brainstorm, Specify, Domain Review, Implement (21 iterations), Conformance
- **Steps skipped**: Spec Merge (not applicable — single direction)
- **Models used**: Opus 4.6 (all steps except implement), GPT 5.4 (implement)
- **Conversation histories available**: Research (autonomous, no interaction), Braindump, Brainstorm, Specify, Domain Review

---

## Phase 2 — Per-Step Evaluation

### Step 1: Research (`research-v2.md`)

#### Output Quality
- **Completeness**: All 9 streams + synthesis produced. Every section the prompt requires is present. 3,550 lines total across 10 files.
- **Specificity**: CONCRETE. Real project names with star counts (Langfuse 24.3k, Promptfoo 19.1k), URLs, specific architectural patterns, actual acquisition data (Promptfoo to OpenAI $86M). Weakest streams: 04-architecture.md (161 lines, patterns described at high level) and 07-trajectory.md (155 lines, predictions directional but not well-sourced).
- **Grounding**: Strong. ~30 web searches and ~20 web fetches per transcript. Sources sections present in each stream. 5-claim sample: 3 of 5 directly verifiable via URLs. The 57% org stat and some acquisition figures lack inline source URLs despite the prompt requiring them.
- **Internal consistency**: Good. Terms used consistently across streams. Synthesis correctly reflects individual stream findings. No cross-stream contradictions.

#### Downstream Utility
- **Actually used**: 02-domain-language.md (braindump, specify, domain review), 01-landscape.md (braindump, specify), 09-failure-archaeology.md (brainstorm, specify), 06-ecosystem.md (braindump, specify, domain review), synthesis.md (brainstorm), 05-pain-points.md (brainstorm, braindump).
- **Produced but not used**: 08-community.md adoption metrics, business models, key people — context-dependent (useful for product planning, low-value for personal projects). 07-trajectory.md predictions — downstream gap (brainstorm should check trajectory signals). Parts of 03-user-workflows.md — 5 of 6 segments not directly consumed, but user personas are strategically important as use-case sources and future review-agent inputs.
- **Needed but not provided**: Concrete end-to-end usage examples for existing tools (how someone actually runs an eval with Braintrust today). Comparative analysis of API designs across projects (function signatures, builder patterns, configuration approaches).

#### Interaction Quality
Autonomous session — 0 interaction rounds. By design (research-v2.md is the only non-interactive prompt). Appropriate for research, but no opportunity to redirect mid-research or add projects discovered during early streams.

#### Prompt Effectiveness
- **Instructions followed**: Faithful. All streams, checkpoint messages, synthesis sections.
- **Constraints respected**: Sources sections present. No obvious fabrication.
- **Failure modes**: Uneven depth — Stream 1 (landscape, 1,480 lines) is nearly half the total output. Streams 7-9 show less search breadth. No length guidance in prompt.
- **Prompt gaps**: No mid-point user checkpoint. No instruction to extract concrete examples or compare API designs. No depth adjustment based on project context (personal vs product). 08-community.md produces full analysis regardless of project type.

---

### Step 2: Braindump (`braindump.md`)

#### Output Quality
- **Completeness**: All 4 files produced. All prompt-required sections present including all 12 Extracted Elements sub-categories.
- **Specificity**: CONCRETE. IDs assigned systematically (I-01 through I-07, H-01 through H-05, T-01 through T-06, Q-01 through Q-08, A-01 through A-05). Conviction levels assigned. Cross-references cite specific research streams.
- **Grounding**: Strong. Every cross-reference claim cites a research stream. Three web searches performed via parallel subagents for gaps (agentevals-dev internals, Rust eval tools, OTel correlation patterns).
- **Internal consistency**: Good. One minor issue: A-04 references "Stream 6, Constraint 4" — a citation format inconsistent with other references.

#### Downstream Utility
- **Actually used**: I-xx IDs in brainstorm clustering. 01-cross-reference.md analysis by brainstorm Phase 3. Synthesis exploration areas for brainstorm cluster formation.
- **Produced but not used**: Q-xx answered questions — never revisited downstream. A-xx load-bearing assumptions — not tracked as running register. 02-discoveries.md generative ideas (cargo test for evals, Scorer Composition Algebra, Schema-Validated Scoring) — should have been actively explored by brainstorm.
- **Needed but not provided**: Concrete evaluation scenarios. Decision rationale for early choices (T-01 AI-agnostic, T-02 Rust — the "why" is thin, requiring brainstorm to re-explore).

#### Interaction Quality
- **Best questions**: "Concrete example of 'low-level'?" (scorer abstraction definition), "What triggered this exploration?" (core pain points), "Storage: querying vs reproducing?" ("delay storage" decision).
- **Worst questions**: "Who is this for besides you?" (obvious), "Resource situation?" (dismissed), "Have you shipped FFI bindings before?" (tangential).
- **User engagement**: Substantive throughout. Strongest contribution: post-Round 3 correction distinguishing integrated mode from observation mode.
- **Confirmation gates**: Worked. Phase 4.5 checkpoint productive (correlation relaxation, cost tracking reframe).
- **Course corrections**: 3, all productive (loop framing, correlation, cost tracking).
- **Round count**: 6 rounds. Appropriate (within 3-5 target, extended by productive correction).

#### Prompt Effectiveness
- **Instructions followed**: All 6 phases executed. Record format matches requirements.
- **Constraints violated**: "Challenge strong opinions with evidence" — partially. T-02 (Rust) not challenged despite Python SDK being "disqualifying for most use cases" per research.
- **Failure modes**: Under-challenging — agent acts as supportive organizer rather than critical interrogator.
- **Prompt gaps**: No instruction to capture concrete use case scenarios. No instruction to update synthesis when downstream steps override it. No guidance on subagent use for parallel cross-referencing.

---

### Step 3: Brainstorm (`brainstorm.md`)

#### Output Quality
- **Completeness**: Both outputs produced. All phases executed. All required sections per direction present in directions.md.
- **Specificity**: MIXED. Concrete: MVP scope with specific deliverables, kill criteria. Vague: distribution sections (channel lists rather than strategies), Direction 3 growth path steps.
- **Grounding**: Each direction cites specific research streams and braindump IDs. Failure cross-checks reference 09-failure-archaeology.md.
- **Internal consistency**: Two stale references (SpanExtractor, Correlator) introduced here but removed during specify. Expected — no feedback loop.

#### Downstream Utility
- **Actually used**: Direction 1 structure consumed by specify Phase 1. Kill criteria informed user decisions. Identity reframe used as design principle.
- **Produced but not used**: Distribution/sustainability sections (appropriate for personal project). Directions 2 and 3 as standalone entities (unified in specify). Validation plans (cheapest tests) — never executed.
- **Needed but not provided**: Concrete test scenarios. Terminology decisions (explicitly deferred). Engagement with braindump generative ideas.

#### Interaction Quality
- **Best questions**: "What does success in 3 months look like?" (clearest metric), "If only ONE cluster?" (revealed hedging), "What specifically is muddy about the core?" (named terminology problem).
- **Worst questions**: "Does the Promptfoo vacuum matter?" (obvious no), "What if Rust is wrong?" (already settled), "What would make this a waste of time?" (circular answer).
- **User engagement**: Mixed. Strong on architectural decisions, weak on variant selection (Core x3 with declining elaboration).
- **Course corrections**: 3 (traceparent as implementation detail, design principles not OTel-tailored, terminology deferred).
- **Round count**: ~10 rounds. Slightly high — cluster-by-cluster exploration produced diminishing returns.

#### Prompt Effectiveness
- **Instructions followed**: All 5 phases executed. Decisions-log captures all required categories.
- **Constraints respected**: Kill criteria not skipped. Every direction has first-100-users answer (shallow but present).
- **Failure modes**: Confirmatory exploration (user had already decided). Redundant questioning on settled decisions.
- **Prompt gaps**: No early-convergence detection. No instruction to engage with braindump generative ideas. No concrete test scenario capture. No subagent guidance for parallel cluster exploration.

---

### Step 4: Specify (`specify.md`)

#### Output Quality
- **Completeness**: Full 14-section spec (1,150 lines). 13 user stories. 13 architectural decisions. Full Rust API. Decisions-log (433 lines). Most complete artifact in the pipeline.
- **Specificity**: CONCRETE. The strongest output. Full Rust type signatures. Binary pass/fail acceptance criteria. Named statistical methods. Specific error codes. 6 open questions with impact and deadline, not vague TBDs.
- **Grounding**: Architectural decisions cite research streams. Failure archaeology cross-check table present. Ecosystem alignment verified.
- **Internal consistency**: Excellent after code-reviewer subagent fixed 16+ issues. Terms match Round 0 term sheet. Only residual: decisions-log captures SampleReport/SampleResult oscillation.

#### Downstream Utility
- **Actually used**: Entire spec consumed by implementation. All ADs checked by conformance. Glossary consumed by domain review.
- **Produced but not used**: Security Considerations (Section 10) not referenced by implementation or conformance. Growth Path Compatibility not validated.
- **Needed but not provided**: Explicit implementation ordering guidance (iteration 13 BLOCKED on dependency). Test strategy (implementation had to invent).

#### Interaction Quality
- **Best questions**: API stress tests against real workflows (highest-value technique in pipeline). Direction enum brainstorm. Mapper naming exploration.
- **Worst questions**: Hard to identify genuinely low-value questions in this step.
- **User engagement**: Highest in pipeline. User actively co-designing: 5 API redesign proposals, transform/reference unification, measurement analogy, module layout, error handling simplification. Only step where user generated more design ideas than agent.
- **Course corrections**: 9+ corrections, all improving the design.
- **Round count**: ~16 rounds. Appropriate — every round produced substantive decisions.

#### Prompt Effectiveness
- **Instructions followed**: All phases and rounds executed. Term sheet locked. Validation cross-checks performed.
- **Constraints respected**: Vague requirements challenged. No TBDs in final spec. All ADs reference research.
- **Failure modes**: Passive API design (user had to say "it's your job" to trigger stress-testing). Terminology oscillation (SampleReport/SampleResult) despite Round 0 gate.
- **Prompt gaps**: No mandate to proactively stress-test designs. No self-review subagent instruction. No Round 0 enforcement mechanism for subsequent rounds. No test strategy output. Decisions-log format not standardized.

---

### Step 5: Domain Review (`spec-domain-review.md`)

#### Output Quality
- **Completeness**: Both outputs produced. All 5 phases executed. 93 terms extracted, 57 analyzed in detail. Glossary rewrite: 49 entries up from 37.
- **Specificity**: CONCRETE. Every term has status, severity, cross-reference to research.
- **Grounding**: Every assessment cites research documents. Metric collision finding cites DeepEval and RAGAS specifically.
- **Internal consistency**: Clean.

#### Downstream Utility
- **Actually used**: Glossary rewrite replaced spec Section 14. 8 prose standardizations applied.
- **Not used**: Full analysis is a one-time audit artifact. Appropriate.
- **Needed**: Nothing missing.

#### Interaction Quality
- 5 rounds, all minimal confirmations. Appropriate for review step.
- No course corrections needed — review was clean.

#### Prompt Effectiveness
- **Instructions followed**: All 5 phases executed faithfully.
- **Constraints respected**: No scope creep. Research-sourced recommendations.
- **Failure modes**: Possible under-severity — 0 high-severity across 93 terms. The Round 0 terminology gate may have done most of the heavy lifting, making this step confirmatory.
- **Prompt gaps**: Structural question — if domain review rarely produces high-impact findings because Round 0 catches most issues, consider integrating it as a specify post-write sub-phase rather than a separate step.

---

### Step 6: Implement (`implement.md` + GPT 5.4)

#### Output Quality
- **Completeness**: 20 components, 21 iterations, 18 source modules (4,959 lines), 14 test files (2,510 lines), 77 tests, 0 failures.
- **Specificity**: CONCRETE. State file has full per-component records with interfaces, ACs, deviations, and cross-iteration notes.
- **Grounding**: Every decision traces to spec. Deviations cite specific spec sections.
- **Internal consistency**: Excellent. BLOCKED mechanism worked correctly (iteration 13).

#### Downstream Utility
- **Actually used**: Source code and state file consumed by conformance.
- **Needed but not provided**: Consolidated conformance mapping. Explicit deviation cross-reference for conformance step.

#### Interaction Quality
No conversation transcripts available for implementation iterations. State file provides indirect evidence of successful execution.

#### Prompt Effectiveness
- **Instructions followed**: 7-step iteration protocol followed across 21 iterations. One-component-per-iteration respected.
- **Constraints respected**: No placeholders. State file maintained. Tests load-bearing. Exit clean.
- **Failure modes**: Iteration 13 BLOCKED (spec dependency ordering gap). 5 deviations all Rust type-system constraints (model mismatch between spec author and implementer).
- **Prompt gaps**: No use-case validation. No conformance-ready summary output. No subagent guidance. No guidance on how loop mechanism maintains context.

---

### Step 7: Conformance

#### Output Quality
- **Completeness**: 7 assessment categories, per-AC verdicts with file/line evidence, deviation reports, recommendations.
- **Specificity**: CONCRETE. Quantified conformance (~95%). Severity-rated deviations. Split recommendations.
- **Grounding**: Every verdict cites source files and line numbers.
- **Internal consistency**: Good. Honest about limitations (2 NOT ASSESSABLE constraints, linker issue preventing test execution).

#### Downstream Utility
Terminal step. Feeds into retrospective.

#### Prompt Effectiveness
Prompt not available — evaluated from output only.
- **Strengths**: Structured per-category approach. Calibrated severity. "Notable Implementation Qualities" section (positive findings most audits skip).
- **Failure modes**: Incomplete deviation capture — 3 deviations found vs 5 in state.md, zero overlap. State file not consulted. Security considerations not verified.
- **Prompt gaps**: No cross-reference with state.md deviation log. No security audit. No behavioral (use-case) conformance, only structural.

---

## Phase 3 — Handoff Evaluation

### Handoff: Research -> Braindump

- **Format compatibility**: Clean. File paths, section structure, all match.
- **Information transfer**: High fidelity. Landscape, domain language, failure archaeology, ecosystem all consumed. Lost: 08-community.md and 07-trajectory.md not engaged with (downstream gap — braindump prompt doesn't instruct trajectory/community checks). Healthy sharpening from broad landscape to user's specific interests.
- **Scope**: Narrowed (healthy). "65+ project landscape" to "Rust eval library with OTel observation." User-driven.

### Handoff: Braindump -> Brainstorm

- **Format compatibility**: Minor friction. Brainstorm prompt doesn't specify how to handle generative ideas in 02-discoveries.md.
- **Information transfer**: Some loss. Generative ideas dropped. Q-xx answered questions not revisited. A-xx assumptions not tracked forward. Braindump synthesis build order overridden but not updated.
- **Scope**: Narrowed (healthy). "7 ideas, 5 hypotheses, 6 technical opinions" to "3 directions from 6 clusters."

### Handoff: Brainstorm -> Specify

- **Format compatibility**: Minor friction. Decisions-log (110 lines) thin for what specify needs. No formal direction IDs for cross-referencing.
- **Information transfer**: Some loss. Kill criteria not formally checked. Validation plans not referenced. Design spike not executed. SpanExtractor/Correlator concepts introduced here, removed in specify, brainstorm not updated.
- **Scope**: Narrowed and unified (healthy). "3 directions" to "1 unified spec with feature-gated extensions."

### Handoff: Specify -> Domain Review

- **Format compatibility**: Clean.
- **Information transfer**: High fidelity. All 93 terms extracted. Round 0 baseline available. Research corpus for cross-reference.
- **Scope**: Stable. Terminology audit doesn't change scope.

### Handoff: Specify -> Implement

- **Format compatibility**: Minor friction. 1,150-line spec navigated by each fresh session. No per-component briefs.
- **Information transfer**: High fidelity (spec to code direction). No transfer in code to spec direction (no feedback loop). Open questions not resolved pre-implementation. Security considerations not referenced.
- **Scope**: Stable. Faithfully implemented.

### Handoff: Implement -> Conformance

- **Format compatibility**: Minor friction. State file not consumed by conformance.
- **Information transfer**: Some loss. State file's 5 deviations (DEV-01 through DEV-05) and conformance's 3 deviations (D-01 through D-03) are completely disjoint. Neither stage alone captures all deviations. Combined they found 8 unique issues.
- **Scope**: Stable.

### Scope Trajectory

```
Research:       "LLM/AI evaluation frameworks, tools, and practices across 65+ projects"
-> Braindump:    "Rust eval library, domain-agnostic core, OTel observation, layered architecture"
-> Brainstorm:   "3 directions narrowed from 6 clusters: Eval Kernel, Trace Grader, Confident Eval"
-> Specify:      "Unified crate: 13 user stories, 13 ADs, full Rust API with feature-gated extensions"
-> Domain Review: "Terminology validated: 93 terms, 0 high-severity issues"
-> Implement:    "20 components, 77 tests, 5 implementation-level deviations"
-> Conformance:  "~95% conformance, 3 minor deviations"
```

Assessment: Healthy funnel. Broad to narrow to concrete. Each narrowing driven by user decisions, not agent drift. No scope oscillation, no premature narrowing, no drift. One structural concern: no backward flow. Stale artifacts accumulate as downstream steps refine upstream decisions.

---

## Phase 4 — Cross-Cutting Analysis

### Waste Analysis

- **Research sections never cited**: 08-community.md (context-dependent — useful for product, not personal project), 07-trajectory.md predictions (downstream gap), 04-architecture.md anti-patterns (downstream gap — specify validation doesn't check them).
- **Braindump ideas never explored**: 02-discoveries.md generative ideas (should have been used — brainstorm should engage with them). Q-xx answered questions not revisited. A-xx load-bearing assumptions not tracked.
- **Brainstorm clusters set aside**: Clusters 4-6 (CLI, Artifact, Self-Aware) — minimal exploration, clear verdicts, acceptable. Directions 2-3 as standalone — unified in specify, ~30% of brainstorm output became irrelevant.
- **Spec sections revised during implementation**: Very low — ~95% conformance. Spec survived implementation remarkably intact.
- **Total waste estimate**: 15-20% of pipeline output unused downstream. Largest contributors: research streams 7-8 (~330 lines), brainstorm distribution/growth for merged directions (~250 lines). Actionable reduction is in downstream engagement (make brainstorm consume trajectories, generative ideas, anti-patterns), not in producing less upstream.

### Consistency Chain

| Decision | First Appeared | Research Basis | Braindump Ref | Brainstorm Ref | Spec Section | Implementation | Conforming? |
|----------|---------------|----------------|---------------|----------------|-------------|----------------|-------------|
| Generic Scorer trait | Braindump T-01 | 04-architecture patterns | T-01 firm conviction | Direction 1 core decision | AD-01, US-01 | src/scorer.rs | Yes |
| Score as enum | Braindump I-04 | 02-domain-language types | I-04 directional | Direction 1 decision | AD-03, Section 6 | src/score.rs | Yes |
| OTel observe (framework doesn't call agent) | Braindump I-03 | 01-landscape agentevals-dev | I-03 strong conviction | Direction 2 | AD-05, US-07 | src/acquisition.rs + src/otel.rs | Yes |
| Single crate, feature-gated | Brainstorm Phase 4 | 04-architecture library pattern | Not in braindump | Decisions-log | AD-12, Section 11 | Single crate | Yes (minor D-03) |
| Mapper trait (unified transforms) | Specify Round 2 | 02-domain-language, 04-architecture | I-06 vague hunch | Not in brainstorm | AD-06, US-03 | src/mapper.rs | Yes |

Decisions 1-3: intact chains. Decision 4: partial chain (emerged in brainstorm). Decision 5: late-forming (vague hunch to elegant abstraction during specify co-design — the pipeline's best abstraction emerged from user interaction, not structured analysis).

### User Bottleneck Analysis

**Couldn't answer**: Post-processing transforms (premature — needed design exploration), external inspirations (genuinely unanswerable), statistical rigor in Score types (premature — belongs in specify), "if only ONE cluster" (productive uncertainty revealing convergence).

**Answered differently than expected**: "What if Rust is wrong?" dismissed as wasteful (settled decision re-asked). "Does the API look right?" redirected to "it's your job" (agent too passive). "Foundation or tool?" reframed entire project (most important brainstorm question).

**Fatigue signals**: Brainstorm rounds 4-6 (Core x3, declining elaboration). Specify rounds 7 and validation (terse confirmations on clean sections). Domain review throughout (appropriate for approver role).

### Model Performance

Opus 4.6: good fit for all assigned steps. One failure mode: passive design in specify (needed user push to stress-test). Better addressed by prompt improvement than model switching.

GPT 5.4: good fit for implementation. 5 justified deviations (Rust type-system constraints). 77 tests, 0 failures across 21 iterations.

No model routing changes recommended.

### Process Gaps

**Missing steps**:
1. Use Case / Scenario Definition step between brainstorm and specify — extract, validate, document concrete evaluation scenarios as validation anchors for entire downstream pipeline.
2. Design Spike / Prototype step between brainstorm and specify — validate key design decisions cheaply before full specification.
3. Transcript Saving as final task in every step — save conversation history to structured markdown.

**Missing feedback loops**:
1. Specify to brainstorm: concept removals (SpanExtractor, Correlator) should update upstream artifacts.
2. Implement to spec: DEV-xx deviations should flow back as spec errata.
3. Conformance to spec: D-xx deviations should trigger spec clarification or implementation fixes.
4. Conformance to state file: cross-reference both deviation sets.

**Missing validation**:
1. Load-bearing assumptions never formally validated (A-01, A-03).
2. Open questions not resolved pre-implementation (OQ-06 design spike never executed).
3. No end-to-end use-case validation of final implementation.

**Parallelization opportunities**:
1. Research streams 4-9 (independent of each other, depend only on 1-3).
2. Brainstorm cluster exploration (independent per cluster).
3. Conformance + retrospective prep.

Note: User explicitly prefers implementation to not start until all planning/review steps complete.
