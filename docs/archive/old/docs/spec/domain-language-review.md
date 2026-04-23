> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Domain Language Review: The Eval Kernel

**Date**: 2026-04-03
**Spec reviewed**: `spec/eval-kernel.md`
**Research corpus**: `research/01-landscape.md`, `research/02-domain-language.md`, `research/03-user-workflows.md`, `research/04-architecture.md`, `research/06-ecosystem.md`
**Brainstorm**: `brainstorm/directions.md`
**Decisions log**: `spec/decisions-log.md`

---

## Domain

This specification operates in the **AI/LLM Evaluation** domain — tools, frameworks, and libraries for evaluating AI model outputs, agent behavior, correctness, and performance. It intersects with **OpenTelemetry observability** (traces, spans) and **applied statistics** (confidence intervals, significance testing for non-deterministic systems).

---

## Phase 1 — Extract

93 domain terms extracted across 14 sections of the specification. Organized into:
- **API Surface**: 27 terms (Sample, Dataset, Score, Scorer, Mapper, Acquisition, Run, etc.)
- **Data Model**: 32 terms (Numeric, Binary, Label, Metric, TrialResult, SampleResult, Change, etc.)
- **Error & Status**: 17 terms (RunBuildError, AcquisitionError, ScorerError, etc.)
- **Config & Integration**: 12 terms (otel, llm-judge, JSONL, correlation_id, etc.)
- **Conceptual**: 27 terms (evaluation, scorer, trial, trajectory, LLM-as-judge, multi-trial, etc.)

Some terms appear in multiple categories (e.g., `Scorer` is both an API surface trait and a conceptual term).

---

## Phase 2 — Cross-Reference Against Research

### Term Reference Map — Core Domain Concepts

| Spec Term | Research Term(s) | Source | Match? | Notes |
|-----------|-----------------|--------|--------|-------|
| `Sample` | "Task" (Inspect AI, Anthropic), "Test Case" (DeepEval), "Sample" (RAGAS) | 02-domain-language §Task/Test Case | partial | Domain fractured (High). Follows RAGAS. Glossary documents. |
| `Dataset` | "Dataset" (universal) | 02-domain-language §Dataset | ✅ match | Standard. |
| `reference` | "Ground Truth" (Microsoft), "Reference" (RAGAS), "Target" (Inspect AI), "Expected Output" (DeepEval) | 02-domain-language §Ground Truth | partial | Domain fractured (High). Follows RAGAS. Glossary documents. |
| `Scorer` | "Grader" (Anthropic), "Scorer" (Inspect AI, Braintrust), "Evaluator" (LangChain, Arize), "Metric" (DeepEval) | 02-domain-language §Grader/Scorer/Evaluator | partial | Domain's most fragmented term (Very High). Follows Inspect AI/Braintrust. |
| `Score` | "Score" (universal) | 02-domain-language §Metric/Score | ✅ match | Universal pairing with Scorer. |
| `evaluation`/`eval` | "Evaluation"/"Eval" (universal) | 02-domain-language §Evaluation | ✅ match | Settled. |
| `Trial` | "Trial" (Anthropic, Agentrial) | 02-domain-language §Trial | ✅ match | Formal term. |
| `Run` | "Run" (common informal) | 02-domain-language §Trial | ✅ match | Standard. |
| `Trace` | "Trace" (OTel standard) | 02-domain-language §Trace; 06-ecosystem | ✅ match | OTel standard. |
| `Span` | "Span" (OTel standard) | 02-domain-language §Span; 06-ecosystem | ✅ match | OTel standard. |
| `Trajectory` | "Trajectory" (LangChain, Anthropic) | 02-domain-language §Trajectory | ✅ match | Domain convention. |
| `pass_at_k` | "pass@k" (HumanEval paper) | 02-domain-language §pass@k | ✅ match | Settled. |
| `pass_all_k` | "pass^k" (Anthropic) | 02-domain-language §pass@k | ✅ match | More readable variant. |
| `baseline` | "baseline" (criterion, benchstat) | Decisions-log Round 3 | ✅ match | Standard. |
| `candidate` | "candidate" (A/B testing) | Decisions-log Round 3 | ✅ match | Standard. |
| `LLM-as-judge` | "LLM-as-a-Judge" (settled, Wikipedia-notable) | 02-domain-language §LLM-as-a-Judge | partial | Drops article "a". Not in glossary. |

### Term Reference Map — Score Types

| Spec Term | Research Term(s) | Source | Match? | Notes |
|-----------|-----------------|--------|--------|-------|
| `Numeric` | No standard variant name | — | N/A | Novel. |
| `Binary` | Pass/fail concept | 02-domain-language | ✅ match | Standard concept. |
| `Label` | No standard | — | N/A | Novel. |
| `Metric` (Score variant) | "Metric" = scorer (DeepEval, RAGAS) | 02-domain-language §Metric/Score, §Grader | ⚠️ conflict | Collision with domain's use of "Metric" for scorer component. |

### Term Reference Map — Architecture Concepts

| Spec Term | Research Term(s) | Source | Match? | Notes |
|-----------|-----------------|--------|--------|-------|
| `Acquisition` | No domain equivalent | — | N/A | Novel abstraction. |
| `inline` | No standard (implicit default) | — | N/A | Novel name for unnamed default. |
| `observe` | No standard (agentevals-dev unnamed) | 01-landscape §agentevals-dev | N/A | Novel. |
| `Mapper` | No eval-domain equivalent | — | N/A | Novel in eval, familiar from FP. |
| `ScorerSet` | No domain equivalent | — | N/A | Novel. |
| `ScorerContext` | No standard | — | N/A | Novel. |
| `ScoreDefinition` | No standard | — | N/A | Novel. |
| `Direction` | No standard (implicit) | — | N/A | Novel formalization. |
| `RunResult` | No standard as raw-data concept | — | N/A | Novel distinction from stats. |
| `TraceBackend` | No standard | — | N/A | Novel. |
| `Change` | No standard | — | N/A | Novel. |
| `Comparison` | No standard as type | — | N/A | Novel. |
| `correlation_id` | No standard eval term | — | N/A | Novel. |

### Glossary Cross-Check

**No term sheet drift detected** in existing glossary entries — spec body uses all glossary terms consistently.

**10 glossary gaps identified** — terms used in spec body but absent from glossary:
1. `LLM-as-a-Judge` (core domain concept, used in US-05, §5.3, §7)
2. `TrialResult` (data model §6.1)
3. `SampleResult` (data model §6.1)
4. `RunMetadata` (data model §6.1)
5. `ScorerComparison` (data model §6.1)
6. `SampleComparison` (data model §6.1)
7. `TraceBackendError` (error type §5.4)
8. `SpanEvent` (OTel type §5.6)
9. `correlation_id` (observe mode §5.6, §5.7)
10. `Observe` (struct type §5.6)

---

## Phase 3 — Gap Analysis

| # | Term | Status | Issue | Severity |
|---|------|--------|-------|----------|
| 1–25 | (25 terms) | ✅ ALIGNED | — | — |
| 26 | `Metric` (Score variant) | ⚠️ DRIFT | Naming collision with DeepEval/RAGAS "Metric" (means scorer, not measurement) | medium |
| 27 | `llm_judge`/`llm-judge` | ⚠️ DRIFT | Shortened from domain standard "LLM-as-a-Judge" | low |
| 28 | `LLM-as-judge` concept | 📋 TERM SHEET DRIFT | Used 4+ times in spec body, no glossary entry | medium |
| 29–50 | (22 terms) | 🆕 NOVEL | All justified — genuinely new abstractions with no domain equivalent | low |
| 51–52 | `TrialResult`, `SampleResult` | 📋 TERM SHEET DRIFT | In data model but missing from glossary (RunResult IS there) | medium |
| 53–56 | `RunMetadata`, `ScorerComparison`, `SampleComparison`, `TraceBackendError` | 📋 TERM SHEET DRIFT | In data model/errors but missing from glossary | low |
| 57 | `correlation_id` | 📋 TERM SHEET DRIFT | Central to observe mode, no glossary entry | medium |

### Summary

| Status | Count | High | Medium | Low |
|--------|-------|------|--------|-----|
| ✅ ALIGNED | 25 | 0 | 0 | 0 |
| ⚠️ DRIFT | 2 | 0 | 1 | 1 |
| ❌ MISNAMED | 0 | 0 | 0 | 0 |
| 🔀 INCONSISTENCY | 0 | 0 | 0 | 0 |
| 🔍 MODELING ISSUE | 0 | 0 | 0 | 0 |
| 🆕 NOVEL TERM | 23 | 0 | 0 | 23 |
| 📋 TERM SHEET DRIFT | 7 | 0 | 3 | 4 |
| **TOTAL** | **57** | **0** | **4** | **28** |

---

## Phase 4 — Recommendations

### ⚠️ DRIFT

#### `Metric` (Score variant) — no rename, documentation fix

- **Severity**: medium
- **Issue**: "Metric" in DeepEval/RAGAS means the scorer component. In this project it's a Score variant.
- **Recommendation**: Do not rename. Strengthen the glossary warning from "Potential confusion" to an explicit domain collision note with a concrete mapping example (DeepEval `Metric` = this project's `Scorer`; DeepEval metric output = this project's `Score::Metric`).

#### `llm_judge` / `llm-judge` — no rename, prose consistency fix

- **Severity**: low
- **Issue**: Domain standard is "LLM-as-a-Judge" (with article). Spec prose varies.
- **Recommendation**: Do not rename function/feature gate. Use canonical "LLM-as-a-Judge" consistently in all prose and documentation.

### 📋 TERM SHEET DRIFT

#### Add glossary entry: `LLM-as-a-Judge`

Core domain concept used in US-05, §5.3, §7, and feature gate — no glossary entry exists. Add with definition, note on Rust casing conventions for function/feature names.

#### Add glossary entries: `TrialResult`, `SampleResult`

Data model entities at the same granularity as `RunResult` (which IS in the glossary). Users encounter these in every API interaction. Add for consistency.

#### Add glossary entry: `correlation_id`

Central to observe mode. Users configuring observe mode must understand this concept and how it differs from OTel trace IDs. Add with explanation of domain-level vs. OTel-level correlation.

#### Add glossary entries: `RunMetadata`, `ScorerComparison`, `SampleComparison`, `TraceBackendError`

Minor gaps. Add brief entries for completeness.

### 🆕 NOVEL TERMS — All Justified

All 23 novel terms are genuinely new abstractions with no domain equivalent:
- **Acquisition/inline/observe**: Names previously-unnamed concepts
- **Mapper/ScorerSet/ScorerContext**: Structural types without domain precedent
- **ScoreDefinition/Direction/Change**: Formalizes implicit domain concepts
- **RunResult/RunStats/ScorerStats**: Novel raw-data vs. computed-stats separation
- **TraceBackend/Comparison/correlation_id**: Novel observe-mode abstractions
- **Numeric/Label**: Score variant names where domain has none
- **Error types**: Follow compositionally from parent concepts

No novel term requires renaming.

---

## Overall Assessment

**This is an exceptionally clean specification from a domain language perspective.**

The Round 0 terminology gate was highly effective — every top-level entity name is either aligned with domain convention or is a documented deliberate choice in a fractured domain. The spec has:

- **Zero high-severity issues**
- **Zero misnamed terms**
- **Zero internal inconsistencies**
- **Zero modeling issues**
- 4 medium-severity items, all glossary gaps (not naming errors)
- 23 novel terms, all justified by genuinely new abstractions

The only action items are:
1. Add 10 glossary entries (low effort, ~1 paragraph each)
2. Strengthen one existing glossary note (Metric collision)
3. Standardize prose usage of "LLM-as-a-Judge" (find-and-replace)
