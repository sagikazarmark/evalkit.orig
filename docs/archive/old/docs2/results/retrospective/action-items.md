> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Pipeline Retrospective — Action Items

## Meta
- **Project**: evalkit (Eval Kernel direction)
- **Date**: 2026-04-04
- **Pipeline version**: research-v2.md, braindump.md, brainstorm.md, specify.md, spec-domain-review.md, implement.md, conformance (prompt not available)
- **Steps executed**: Research, Braindump, Brainstorm, Specify, Domain Review, Implement (21 iterations), Conformance
- **Conversation histories available**: Research (autonomous), Braindump, Brainstorm, Specify, Domain Review

---

## Per-Step Improvements

### Stage 1: Research (`research-v2.md`)

#### Prompt Changes

1. **Change**: Add a "Project Context" input parameter (personal/OSS/commercial) in the Setup section (lines 3-7). Add conditional depth guidance: "If personal project, reduce Stream 8 (Community) to a lightweight scan (top 5 projects' adoption signals only). If commercial, full depth."
   - **Evidence**: 08-community.md (176 lines) was never cited downstream for this personal project. Business models, contributor diversity, and key people analysis had zero downstream impact.
   - **Expected effect**: ~100 lines less output for personal projects. No loss for product projects.

2. **Change**: In Stream 3 (User Workflows), add instruction after the Jobs-to-be-Done section (around line 180): "For each job, write one concrete end-to-end scenario (named person, specific tool, step-by-step actions, what they see, what goes wrong). These scenarios will serve as validation anchors for downstream pipeline stages."
   - **Evidence**: No concrete scenarios existed. Brainstorm and specify had to invent stress-test workflows ad hoc. User observation: "Example use cases should be part of the process MUCH sooner."
   - **Expected effect**: 5-8 concrete scenarios available from Stage 1, usable as validation anchors through every subsequent stage.

3. **Change**: In Stream 1 (Landscape), add sub-section after per-project documentation (around line 60): "For the top 5 projects by relevance: extract representative API patterns (function signatures, builder patterns, configuration shape, error handling approach). Present as code snippets or pseudocode."
   - **Evidence**: Specify Round 2 designed the API from first principles. Prior art API analysis would have accelerated this and grounded decisions better.
   - **Expected effect**: Specify Round 2 can reference existing API patterns. Reduces "blank canvas" design effort.

4. **Change**: Add a mid-point user checkpoint after Stream 5. Insert between streams 5 and 6: "Present a brief summary of key findings so far. Ask: 'Based on what I've found, should I adjust focus for the remaining streams? Any projects or topics to add?'"
   - **Evidence**: No opportunity to redirect mid-research. If the user discovers a must-include project from Stream 1-5 findings, there's no way to add it. The 0-interaction design is a strength for efficiency but a weakness for relevance.
   - **Expected effect**: User can course-correct. ~1 round of interaction, minimal time cost.

#### Structural Changes

- Add depth balancing guidance: "No single stream should exceed 30% of total research output. If Stream 1 (Landscape) exceeds this, summarize lower-relevance projects rather than expanding."
  - **Evidence**: Stream 1 was 1,480 lines — 42% of total output. Streams 7-9 showed reduced depth, possibly due to context pressure.

---

### Stage 2: Braindump (`braindump.md`)

#### Prompt Changes

1. **Change**: In Phase 2 (Interrogation), add a dedicated round after the current categories (around line 65): "**Use Case Round**: Ask the user to walk through 2-3 concrete evaluation scenarios they want to support. For each: What is being evaluated? What input does it take? What does a good result look like? What does a bad result look like? Record these as UC-01, UC-02, etc. with the same specificity tracking as ideas."
   - **Evidence**: The braindump captured ideas and technical opinions but no concrete scenarios. Specify stress-tests had to be invented. User observation: "Use cases should be part of the process MUCH sooner."
   - **Expected effect**: 2-3 concrete use cases available from Stage 2, validated against research, usable as anchors through brainstorm, specify, and conformance.

2. **Change**: In Phase 5 (Discover), add instruction (around line 170): "Go through EVERY generative idea. For each: assess feasibility, effort signal, and whether it connects to any user idea or exploration area. Mark each as EXPLORE (should be picked up by brainstorm), DEFER (interesting but not now), or DROP (with reason). Do not silently pass over generative ideas."
   - **Evidence**: 02-discoveries.md generative ideas (cargo test for evals, Scorer Composition Algebra, Schema-Validated Scoring) were never picked up by brainstorm. User explicitly flagged this as important.
   - **Expected effect**: Generative ideas get explicit triage decisions. Those marked EXPLORE flow to brainstorm.

3. **Change**: In Phase 2 (Interrogation), replace the "Resource situation" probe (currently one of the 14 categories, around line 50) with: "**Decision Rationale**: For each strong technical opinion (T-xx), ask: 'Why this choice over the alternatives? What would change your mind?' Record the rationale, alternatives considered, and conditions for revisiting."
   - **Evidence**: "Resource situation" was dismissed outright. T-01 (AI-agnostic) and T-02 (Rust) had thin rationale, causing brainstorm to re-explore ("What if Rust is wrong?" — dismissed as wasteful because the reasoning had already been given but not recorded).
   - **Expected effect**: Early decisions have recorded rationale. Downstream steps don't re-ask settled questions.

#### Structural Changes

- Add "Final task" at the end of Phase 6: "Save the complete conversation history to `transcripts/02-braindump.md` in chronological order, preserving all user messages and agent responses verbatim."
  - **Evidence**: User observation: "Each section/step should have a final task: save conversation history/transcript."

---

### Stage 3: Brainstorm (`brainstorm.md`)

#### Prompt Changes

1. **Change**: In Phase 1 (Map), add instruction after cluster formation (around line 40): "Review `braindump/02-discoveries.md` generative ideas. For each idea marked EXPLORE: assess whether it fits an existing cluster (add it) or forms a new cluster. For ideas not marked: state why they're excluded. Do not silently ignore generative ideas."
   - **Evidence**: Three generative ideas from discoveries were never picked up. User flagged this as a gap.
   - **Expected effect**: Generative ideas are explicitly considered during clustering.

2. **Change**: In Phase 3 (Explore), add after each cluster's sub-sections (around line 130): "**Use Case Validation**: Test this cluster against the use cases from braindump (UC-01, UC-02, etc.). For each use case: does this cluster address it? Partially? Not at all? Are there use cases this cluster enables that weren't in the braindump?"
   - **Evidence**: No use-case validation during brainstorm exploration. Clusters were evaluated on abstract criteria (technical feasibility, market gap) but not against concrete scenarios.
   - **Expected effect**: Each cluster is grounded in real user needs. Dead-end clusters fail the use-case test early.

3. **Change**: In Phase 3 (Explore), add early-convergence detection after the second cluster (around line 135): "If the user has chosen the same variant for 2 clusters: explicitly note the pattern and ask: 'You've picked [variant] for both clusters so far. Should I assume the same for remaining clusters, or do you want to explore them?' If user confirms, skip remaining cluster explorations and proceed to Phase 4."
   - **Evidence**: User picked "Core" for all 3 clusters with declining elaboration. Phase 3 exploration of Cluster 3 produced no new information.
   - **Expected effect**: Saves 1-2 interaction rounds when user preference is clear. Reduces fatigue risk.

4. **Change**: In Phase 4 (Provoke), add instruction before provocations (around line 155): "Skip provocations that challenge decisions the user has stated with 'firm conviction' or 'strong opinion' in the braindump record, UNLESS new evidence from the brainstorm exploration contradicts them. For skipped provocations, state: 'Skipping [provocation] — this was stated with firm conviction in braindump and exploration hasn't contradicted it.'"
   - **Evidence**: "What if Rust is wrong?" was asked despite the user stating T-02 with "firm conviction." Dismissed as "Sounds wasteful." Wasted a round and risked user frustration.
   - **Expected effect**: Provocations focus on genuinely uncertain or newly challenged decisions.

#### Structural Changes

- Add "Final task" at the end of Phase 5: "Save the complete conversation history to `transcripts/03-brainstorm.md`."
- Reduce decisions-log from a separate artifact to a section within directions.md, OR define a consistent ADR format for it (see Pipeline-Level Improvements).

---

### Stage 4: Specify (`specify.md`)

#### Prompt Changes

1. **Change**: In Phase 2, Round 2 (API Surface), add mandatory step after presenting the draft (around line 185): "Before asking the user, stress-test the proposed API yourself against at least 3 real-world workflows. For each workflow: write the code the user would write, identify friction points, and propose improvements. Present both the draft and your stress-test findings."
   - **Evidence**: User had to say "it's kinda your job to find ways to validate the API and make sure it's the best" before the agent stress-tested. The prompt says "challenge vague requirements aggressively" but doesn't say "proactively validate your own proposals."
   - **Expected effect**: Agent arrives at user review having already identified and addressed obvious friction points. Higher-quality first drafts.

2. **Change**: In Phase 2, after the final round and before Phase 3, add: "**Self-Review Pass**: Before presenting to the user, dispatch a subagent to review the accumulated spec content for: internal consistency (do API fields match data model? do errors match taxonomy?), coverage (every user story achievable via defined interfaces?), and terminology consistency (all terms match Round 0 term sheet). Fix issues before presenting."
   - **Evidence**: The code-reviewer subagent at the end caught 16+ issues. These would have been caught earlier with a pre-presentation self-review. User observation: "Subagent review passes: do self-review in all sections."
   - **Expected effect**: Fewer issues reach the user. Higher-quality review experience.

3. **Change**: In Phase 2, Round 0 (Terminology Gate), add enforcement mechanism (around line 115): "Maintain a term watchlist from the confirmed term sheet. In every subsequent round, before presenting content to the user, check that all domain terms match the watchlist. Flag any drift immediately: 'Note: I used [drifted term] but our term sheet says [confirmed term]. Correcting.'"
   - **Evidence**: SampleReport/SampleResult oscillated across rounds despite Round 0 locking SampleResult. The gate works at lock-time but has no enforcement mechanism afterward.
   - **Expected effect**: Terminology stays locked throughout specify.

4. **Change**: In Phase 1 (Anchor), add after the open questions check (around line 30): "**Use Case Anchoring**: Present the use cases from braindump/brainstorm (UC-01, UC-02, etc.). These will serve as validation targets throughout specification. After each round, briefly check: does the current design support all anchored use cases?"
   - **Evidence**: Use cases were not available as validation anchors during specify. The stress-tests in Round 2 were invented ad hoc.
   - **Expected effect**: Every spec round is grounded in concrete scenarios. Regression detection built into the process.

#### Structural Changes

- Mandate a decisions-log output in ADR format (see Pipeline-Level Improvements).
- Add "Final task": save conversation history to `transcripts/04-specify.md`.

---

### Stage 5: Domain Review (`spec-domain-review.md`)

#### Prompt Changes

1. **Change**: In Phase 1 (Extract), add self-review instruction: "After extracting terms, dispatch a subagent to verify the extraction is complete by independently scanning the spec and reporting any terms missed."
   - **Evidence**: User observation on subagent review passes. No evidence of missed terms in this run, but the review's value depends on extraction completeness.
   - **Expected effect**: Higher confidence in extraction completeness.

#### Structural Changes

- Add "Final task": save conversation history to `transcripts/05-domain-review.md`.
- Consider merging into specify as a post-write sub-phase (see Pipeline-Level Improvements, Deferred section).

---

### Stage 6: Implement (`implement.md`)

#### Prompt Changes

1. **Change**: In Step 3 (Scope), add after reading spec sections (around line 58): "Check the spec's use cases (UC-xx) — identify which use cases this component contributes to. After building, verify that the component's interfaces support those use cases."
   - **Evidence**: No use-case validation during implementation. Tests are unit/integration tests, not scenario tests. Conformance audit checks structural conformance but not behavioral conformance.
   - **Expected effect**: Components are validated against real scenarios, not just AC checklists.

2. **Change**: In Step 6 (Record), add after deviation log (around line 110): "**Conformance Preparation**: Maintain a running conformance summary at the end of state.md: a table mapping every US-xx and AC-xx to its implementation status (implemented / deferred / deviated with DEV-xx reference). This table is consumed by the conformance step."
   - **Evidence**: Conformance had to reconstruct the AC-to-implementation mapping. State file tracks ACs per component but doesn't provide a consolidated view.
   - **Expected effect**: Conformance step starts with a pre-built mapping. Faster, more accurate auditing.

#### Structural Changes

- Implementation loop logs (`.loop-logs/`) should be consolidated into a single transcript: `transcripts/06-implement.md` with iteration markers.

---

### Stage 7: Conformance

Note: Conformance prompt not available for review. Recommendations based on output analysis.

#### Prompt Changes (for when the prompt is created/updated)

1. **Change**: Add as a required input: "Read `state.md` deviation log. For each DEV-xx entry, verify whether the deviation is justified and whether downstream components are affected. Cross-reference your own D-xx findings with DEV-xx entries. Report combined deviations."
   - **Evidence**: State file's 5 DEV-xx deviations and conformance's 3 D-xx deviations have zero overlap. Combined: 8 unique issues. Neither alone is sufficient.
   - **Expected effect**: Complete deviation picture. No blind spots.

2. **Change**: Add security audit section: "Verify spec Section 10 (Security Considerations) against implementation. Check trust boundaries, input validation, and threat mitigations."
   - **Evidence**: Security considerations not referenced by implementation or conformance.
   - **Expected effect**: Security requirements verified, not just functional ones.

3. **Change**: Add behavioral conformance section: "For each UC-xx use case: write the code a user would write using the implemented API. Verify it compiles, makes sense, and achieves the use case's goal."
   - **Evidence**: Conformance checks structural conformance (does API match spec?) but not behavioral (does the specified workflow work end-to-end?).
   - **Expected effect**: Use cases validated against real code, closing the validation loop.

---

## Handoff Improvements

### Braindump -> Brainstorm (some loss)
- **Problem**: Generative ideas from 02-discoveries.md silently dropped. Answered questions (Q-xx) not revisited. Load-bearing assumptions not tracked forward.
- **Fix**: 
  - Braindump: mark generative ideas as EXPLORE/DEFER/DROP (braindump.md Phase 5).
  - Brainstorm: add instruction to review and engage with EXPLORE ideas (brainstorm.md Phase 1).
  - Both: maintain an assumption register as a cross-stage artifact (`docs/decisions/assumptions.md`) updated by every stage.
- **Which prompts to edit**: braindump.md Phase 5, brainstorm.md Phase 1 cluster formation section.

### Brainstorm -> Specify (some loss)
- **Problem**: Kill criteria not formally checked. Validation plans not referenced. Design spike not executed. Upstream artifacts become stale.
- **Fix**:
  - Specify Phase 1 (Anchor): add "Check kill criteria — are any triggered? Check validation plan — has the cheapest test been run?"
  - Specify Phase 4 (Write): add "Produce a brief addendum file noting concepts introduced, removed, or renamed vs. brainstorm directions. Save to `stages/04-specify/brainstorm-delta.md`."
- **Which prompts to edit**: specify.md Phase 1 (Anchor section), specify.md Phase 4 (Write section).

### Implement -> Conformance (some loss)
- **Problem**: State file deviation log not consumed by conformance. Two independent deviation sets never cross-referenced.
- **Fix**:
  - Implement: add conformance summary table to state.md (implement.md Step 6).
  - Conformance: add state.md as required input, cross-reference DEV-xx with D-xx.
- **Which prompts to edit**: implement.md Step 6, conformance prompt (to be created/updated).

---

## Pipeline-Level Improvements

### Steps to Add

1. **Use Case Definition** (between Brainstorm and Specify): A focused session where the user defines 3-5 concrete evaluation scenarios (UC-01 through UC-05). Each scenario: what's being evaluated, input shape, expected output, what good/bad looks like, which user persona this serves. These become validation anchors for all downstream stages. Alternatively, use case elicitation can be integrated into braindump (as a dedicated interrogation round) rather than a separate step — the key requirement is that concrete scenarios exist before specify begins.

2. **Transcript Saving** (final task in every step): Each step saves its conversation history to `docs/transcripts/NN-[stage-name].md`. This is not a new step but a new final task added to every existing step's prompt.

### Steps to Split

None recommended. Current step granularity is appropriate.

### Steps to Merge

- **Consider (deferred)**: Domain Review into Specify as a post-write sub-phase. Evidence: Domain review found 0 high-severity issues because Round 0 caught most problems. Counter-evidence: the domain review prompt is sophisticated (5 phases, 93 terms analyzed) and benefits from fresh-context focus. Decision: defer until a second run confirms domain review is consistently low-impact.

### Steps to Remove

- **Spec Merge**: Remove as a standard pipeline step. Make it opt-in with trigger: "Use spec merge only when the brainstorm produces two genuinely independent directions that the user wants to combine into a single project." The brainstorm's convergence pressure (provocation phase, kill criteria) naturally drives toward unity, making merge rarely needed.

### Content to Relocate

- **Use case scenarios**: From ad-hoc invention in specify Round 2 to a structured capture in braindump Phase 2 (or a dedicated step). Use cases should flow forward through brainstorm validation, specify anchoring, implementation verification, and conformance testing.
- **Decision tracking**: From three informal locations (brainstorm decisions-log, spec decisions-log, state.md deviation log) to a unified ADR register (`docs/decisions/`) maintained across all stages (see below).

### Decision Tracking System

Introduce a cross-stage ADR (Architecture Decision Record) system:

**Format per decision:**
```
## ADR-NNN: [Title]
- **Stage**: [where decided]
- **Date**: [when]
- **Status**: proposed | accepted | superseded by ADR-NNN | deprecated
- **Context**: [what prompted this decision]
- **Decision**: [what was decided]
- **Alternatives considered**: [what else was considered and why not]
- **Consequences**: [what follows from this decision]
- **Revisit conditions**: [under what circumstances to reconsider]
```

**Maintained in**: `docs/decisions/` directory, one file per decision or one consolidated file.

**Updated by**: Every stage. Braindump creates initial ADRs for strong technical opinions. Brainstorm adds cluster/direction decisions. Specify adds architectural decisions (AD-xx become ADRs). Implement records deviations as ADR amendments. Conformance cross-references.

### Feedback Loops to Add

1. **Specify -> Brainstorm artifacts**: After specify completes, produce a delta file listing concepts introduced, removed, or renamed vs. brainstorm. This doesn't require re-running brainstorm — it's a documentation update noting what changed and why.

2. **Implement -> Spec errata**: After implementation completes, produce a spec errata file listing all DEV-xx deviations and their impact. The spec itself is not modified (it's a historical artifact), but the errata file documents known divergences for future reference.

3. **Conformance -> Implementation**: Conformance findings that are additive fixes (like D-01 missing prelude, D-02 missing serde derives) should be flagged as immediately actionable. The conformance step should produce a "fix list" that can be fed back to an implementation iteration.

### Parallelization Opportunities

1. **Research streams 4-9**: After foundational streams 1-3 complete, streams 4 (architecture), 5 (pain points), 6 (ecosystem), 7 (trajectory), 8 (community), 9 (failure archaeology) can run as parallel subagents. They depend on having the landscape and domain language established but not on each other.

2. **Brainstorm cluster exploration**: Phase 3 sub-sections (3a-3f) are independent per cluster. With subagent-driven exploration, all shortlisted clusters could be explored in parallel and presented to the user together.

3. **Conformance + retrospective prep**: Retrospective artifact inventory and prompt analysis can begin while conformance runs.

Note: User prefers implementation to NOT start until all planning/review stages complete.

### File Structure

Adopt a lifecycle-stable directory structure from Stage 1:

```
docs/
  prompts/                    # Numbered prompt files
    01-research.md
    02-braindump.md
    03-brainstorm.md
    04-specify.md
    05-domain-review.md
    06-implement.md
    07-conformance.md
    08-retrospective.md
  stages/                     # Output artifacts per stage
    01-research/
    02-braindump/
    03-brainstorm/
    04-specify/
    05-domain-review/
    06-conformance/
    07-retrospective/
  transcripts/                # Conversation histories per stage
    01-research.md
    02-braindump.md
    03-brainstorm.md
    04-specify.md
    05-domain-review.md
    06-implement.md           # Consolidated from loop logs
    07-conformance.md
  decisions/                  # Cross-stage ADRs
    adr-001-rust-core.md
    adr-002-ai-agnostic.md
    ...
src/                          # Implementation (appears when code begins)
tests/
state.md                     # Implementation coordination
```

This structure works from day 1 (only `docs/` exists) through implementation (`src/` added) through maintenance. Planning artifacts never need to be moved.

### Pipeline Terminology

Resolve the "phase" overload:

- **Stage**: Pipeline-level steps (Stage 1: Research, Stage 2: Braindump, ...)
- **Phase**: Within-stage sections (Phase 1: Receive, Phase 2: Interrogate, ... within a stage)
- **Round**: Interactive exchanges within a phase (Round 1, Round 2, ... within Phase 2)

Update all prompt files to use this terminology consistently.

---

## User Interaction Improvements

### Questions to Remove

| Stage | Current Question | Why Low-Value | Action |
|-------|-----------------|---------------|--------|
| Braindump, Phase 2 | "Resource situation?" | Dismissed outright. For personal projects, resource questions are premature. | Remove, or make conditional on project type (product only). |
| Braindump, Phase 2 | "Who is this for besides you?" | Already obvious from context for personal projects. | Replace with: "Walk me through a concrete evaluation you'd want to run." |
| Brainstorm, Phase 4 | Provocations on firm-conviction decisions | Wastes rounds, risks frustration. "What if Rust is wrong?" after 3 Rust commitments. | Skip provocations on braindump items with "firm conviction." |
| Brainstorm, Phase 2 | "Does the Promptfoo vacuum matter?" | User building for self, not market timing. | Remove for personal projects, keep for product projects. |

### Questions to Add

| Stage | Phase | What to Ask | Reason |
|-------|-------|-------------|--------|
| Braindump | Phase 2 (new round) | "Walk through 2-3 concrete evaluation scenarios" | Specify needs use cases as validation anchors |
| Braindump | Phase 2 | "Why this choice over alternatives? What would change your mind?" (per strong T-xx) | Downstream steps re-ask settled decisions without recorded rationale |
| Brainstorm | Phase 3 (per cluster) | "Which use cases does this cluster address?" | Clusters evaluated abstractly, not against concrete needs |
| Specify | Round 2 | Agent self-asks: "What breaks if I stress-test this against real workflows?" | Agent was too passive; user had to push for rigor |

### Interaction Pacing

| Stage | Current | Target | Change |
|-------|---------|--------|--------|
| Braindump | 6 rounds | 5-7 rounds | Add 1 use-case round. Keep overall count similar by making interrogation probes more focused. |
| Brainstorm | ~10 rounds | 7-8 rounds | Add early-convergence exit. Skip redundant provocations. |
| Specify | ~16 rounds | 14-16 rounds | Add self-review pass (reduces back-and-forth). Keep depth — this is the pipeline's most productive interactive stage. |
| Domain Review | 5 rounds | 5 rounds | No change. Appropriate for review/approval. |

### Subagent Usage Guidance

Add to every prompt's preamble (after role statement):

"**Subagent guidance**: For computationally intensive sections (cross-referencing large corpora, exploring independent clusters, reviewing long documents), dispatch parallel subagents rather than processing sequentially. Before presenting any major artifact to the user, dispatch a self-review subagent to check for internal consistency, completeness, and terminology compliance."

Specific subagent opportunities per stage:
- Research: Parallel streams 4-9
- Braindump: Parallel cross-referencing of I-xx, H-xx, T-xx against research
- Brainstorm: Parallel cluster exploration (Phase 3)
- Specify: Self-review before presenting each round; self-review before final write
- Conformance: Parallel category auditing

### Evaluating User Responses

Add to every interactive prompt's preamble:

"**Response quality assessment**: When the user gives a vague, uncertain, or dismissive answer, diagnose why before proceeding:
- **Premature**: Question asked before user has enough context. Note it and revisit later.
- **Unclear**: Question was ambiguous or too abstract. Rephrase with a concrete example.
- **Outside expertise**: User doesn't have this knowledge. Route to research or web search.
- **Decision avoidance**: User hedging on a commitment. Make tradeoffs explicit.
- **Fatigue**: Declining engagement. Offer to summarize, skip, or early-exit.
- **Already answered**: Question covers ground the user already addressed. Acknowledge and move on.

Do not record a vague answer and continue. Adapt your approach based on the diagnosis."

---

## Priority Ranking

### P0 — Fix Before Next Run

1. **Add use case capture to braindump** (braindump.md Phase 2, new Use Case Round) — Use cases are the single most impactful missing input. They ground every downstream stage in concrete scenarios. Without them, specify stress-tests are ad hoc, implementation tests are structural-only, and conformance can't validate behavior.

2. **Establish lifecycle-stable file structure** (all prompts, first lines of Stage 1) — The current structure required manual reorganization when implementation started. Prompts should establish `docs/prompts/`, `docs/stages/`, `docs/transcripts/`, `docs/decisions/` from Stage 1.

3. **Add transcript saving to every stage** (all prompts, final task) — Transcripts are essential for retrospective analysis and currently saved manually. Each prompt's final task should save the conversation history.

4. **Make brainstorm engage with generative ideas** (braindump.md Phase 5 triage + brainstorm.md Phase 1 review) — Generative ideas are currently silently dropped. The braindump should triage them and the brainstorm should explicitly engage with those marked EXPLORE.

5. **Add self-review subagent passes** (specify.md pre-presentation, all stages pre-final-output) — The specify code-reviewer caught 16+ issues post-write. Self-review before presenting to the user reduces back-and-forth and catches consistency issues early.

### P1 — Fix Soon

6. **Add stress-testing mandate to specify** (specify.md Round 2, around line 185) — The agent should proactively stress-test its own API proposals against real workflows, not wait for the user to push for rigor.

7. **Introduce cross-stage decision tracking** (all prompts, new ADR system) — Decisions are currently scattered across 3 files in 3 formats. A unified ADR register maintained from Stage 2 through implementation provides a single decision thread.

8. **Feed state.md into conformance** (implement.md Step 6 + conformance prompt) — Two independent deviation sets (5 DEV-xx + 3 D-xx, zero overlap) are never cross-referenced. Combined, they found 8 unique issues. Each alone missed half.

9. **Resolve pipeline terminology** (all prompts) — "Phase" is overloaded. Adopt: Stage (pipeline-level), Phase (within-stage), Round (interactive exchange).

10. **Number and reorganize prompt files** (file system) — `01-research.md` through `08-retrospective.md` in `docs/prompts/`. Makes ordering explicit, prevents confusion.

11. **Add early-convergence detection to brainstorm** (brainstorm.md Phase 3, after second cluster) — Skip remaining cluster explorations when user preference is consistent. Saves 1-2 rounds, reduces fatigue.

12. **Remove spec merge as standard step** — Make opt-in with explicit trigger criteria. Brainstorm convergence pressure makes it rarely needed.

13. **Track load-bearing assumptions** (braindump.md through specify.md) — A-01 and A-03 were flagged LOAD-BEARING but never formally validated. Add an assumption register as a cross-stage artifact.

### P2 — Improve When Convenient

14. **Skip provocations on firm-conviction decisions** (brainstorm.md Phase 4) — Avoid re-asking settled questions. Check braindump conviction levels before provoking.

15. **Add response quality assessment to interactive prompts** — Diagnose why users give vague answers (premature, unclear, fatigue, already answered) and adapt approach.

16. **Research depth adjustment by project type** (research-v2.md Setup) — Personal projects need less community analysis. Product projects need full depth.

17. **Add mid-point checkpoint to research** (research-v2.md, after Stream 5) — One interaction round to let user redirect focus for remaining streams.

18. **Research API pattern extraction** (research-v2.md Stream 1) — Extract and compare API designs from top projects. Accelerates specify Round 2.

19. **Parallel research streams 4-9** — Independent streams can run as parallel subagents after foundational streams 1-3 complete.

20. **Conformance behavioral testing** — Validate use cases end-to-end against implementation, not just structural conformance.

### Deferred — Needs More Evidence

21. **Merge domain review into specify** — Evidence: 0 high-severity findings in this run because Round 0 caught most issues. Counter-evidence: the domain review prompt is sophisticated and may catch issues Round 0 misses in harder cases. Re-evaluate after next run.

22. **Design spike step between brainstorm and specify** — The brainstorm proposed a scorer trait design spike that was never executed. For projects with novel abstractions, a spike could validate key decisions cheaply. But this run's spec survived implementation intact (~95%), suggesting the specify step was sufficient. Re-evaluate if a future run produces a spec with significant implementation deviations.

23. **User persona review agents** — User mentioned wanting persona agents to review artifacts automatically. This is a pipeline automation improvement, not a prompt improvement. Requires: persona definitions from 03-user-workflows.md, review criteria per persona, integration point (after which stages?). Track adoption of 03-user-workflows.md personas in the next run before building automation.

24. **Feedback loops (specify->brainstorm, implement->spec, conformance->spec)** — The pipeline's waterfall structure worked for this single-pass run. Feedback loops add complexity. If the next run produces more stale-artifact issues, add delta files as the lightest-weight feedback mechanism.

---

## Metrics for Next Run

| Metric | Current Baseline (this run) | Target | How to Measure |
|--------|-----------------------------|--------|----------------|
| Conformance score | ~95% | >95% | Conformance report |
| Research sections cited downstream | 7 of 10 | 9 of 10 | Cross-reference analysis |
| Spec sections revised during implementation | ~5% (5 deviations) | <5% | State file deviation count |
| Interactive rounds: Braindump | 6 | 5-7 | Transcript |
| Interactive rounds: Brainstorm | ~10 | 7-8 | Transcript |
| Interactive rounds: Specify | ~16 | 14-16 | Transcript |
| Use cases defined before specify | 0 | 3-5 | Braindump/brainstorm artifacts |
| Generative ideas engaged by brainstorm | 0 of 3 | all marked EXPLORE | Brainstorm artifact |
| Decisions with recorded rationale | ~50% (informal logs) | >90% (ADR format) | Decision register |
| Combined deviations (state + conformance) | 8 (0 overlap) | 8 or fewer (full overlap) | Cross-reference |
| User "I don't know" responses | 4 | fewer (questions better timed) | Transcript |
| Stages with saved transcripts | 5 of 7 (manual) | 7 of 7 (automatic) | File existence |
| Files manually relocated | yes (prompts, docs) | 0 | User report |
