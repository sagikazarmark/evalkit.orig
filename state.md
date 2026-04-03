# Implementation State

## Meta
- **Spec**: `/home/mark/evalkit/docs/spec/eval-kernel.md`
- **Project root**: `/home/mark/evalkit`
- **Current iteration**: 21
- **Last updated**: 2026-04-03T18:08:54+02:00

## Progress Summary
| Component | Status | Iteration | Tests | Deviations |
|-----------|--------|-----------|-------|------------|
| Sample | DONE | 2 | 4 passed, 0 failed | 1 |
| Dataset | DONE | 3 | 2 passed, 0 failed | 0 |
| Score | DONE | 4 | 4 passed, 0 failed | 0 |
| ScoreDefinition | DONE | 5 | 4 passed, 0 failed | 0 |
| Direction | DONE | 5 | 4 passed, 0 failed | 0 |
| ScorerContext | DONE | 6 | 4 passed, 0 failed | 0 |
| ScorerError | DONE | 7 | 4 passed, 0 failed | 0 |
| Mapper | DONE | 8 | 4 passed, 0 failed | 0 |
| Scorer trait | DONE | 9 | 4 passed, 0 failed | 0 |
| ScorerSet | DONE | 10 | 4 passed, 0 failed | 1 |
| Acquisition trait | DONE | 11 | 4 passed, 0 failed | 0 |
| Built-in scorers | DONE | 12 | 8 passed, 0 failed | 1 |
| Run builder | DONE | 15 | 5 passed, 0 failed | 1 |
| RunResult / SampleResult / TrialResult | DONE | 14 | 4 passed, 0 failed | 0 |
| Stats | DONE | 16 | 5 passed, 0 failed | 0 |
| Comparison | DONE | 17 | 6 passed, 0 failed | 0 |
| JSONL | DONE | 18 | 2 passed, 0 failed | 0 |
| TraceBackend / JaegerBackend [otel] | DONE | 19 | 3 passed, 0 failed | 0 |
| Observe acquisition [otel] | DONE | 20 | 4 passed, 0 failed | 1 |
| LLM-as-a-Judge scorer [llm-judge] | DONE | 21 | 5 passed, 0 failed | 0 |

## Deviation Log

### DEV-01: `SampleBuilder::build()` requires hashable generics
- **Iteration**: 2
- **Component**: Sample
- **Spec says**: `SampleBuilder<I, R>::build() -> Sample<I, R>` with no additional bounds called out in Section 5.1.
- **Actual**: `SampleBuilder::build()` is implemented only for `I: Hash, R: Hash`.
- **Reason**: Builder-created samples auto-generate deterministic IDs when `.id(...)` is not supplied, and generating that ID requires hashing the input and optional reference. Rust cannot express "hash only when no explicit ID was provided" on a single inherent method without splitting the API.
- **Downstream impact**: Future components can rely on the current API for hashable sample types. If later work needs builder support for non-hashable `I`/`R`, the spec will need clarification or the API will need an additional explicit-ID-only construction path.

### DEV-02: `ScorerSet::build()` currently requires `'static` generic types
- **Iteration**: 10
- **Component**: ScorerSet
- **Spec says**: Section 5.5 defines `ScorerSet::builder().map_output(...).map_reference(...).scorer(...).build()` without calling out any `'static` bounds on the generic input/output/reference types.
- **Actual**: `ScorerSetBuilderWithScorers::build()` is implemented only for `'static` `I`, `O`, `R`, and mapped output/reference types.
- **Reason**: The implementation stores heterogeneous async scorers plus optional per-set mappers behind boxed internal executors so one `ScorerSet<I, O, R>` value can erase differing mapped scorer types while still running the mapper exactly once per trial. That internal boxing currently requires owned `'static` generic types.
- **Downstream impact**: `Run` and future scorer-set consumers should use owned types when constructing `ScorerSet`s. If non-`'static` borrowed types need to flow through scorer sets later, the executor storage strategy will need a redesign.

### DEV-03: `json_schema()` currently supports a dependency-free JSON Schema subset
- **Iteration**: 12
- **Component**: Built-in scorers
- **Spec says**: Section 5.3 exposes `json_schema(schema: serde_json::Value) -> impl Scorer<String, String>` without narrowing schema keyword support.
- **Actual**: `json_schema()` validates outputs against a dependency-free subset of JSON Schema keywords: boolean schemas plus `type`, `required`, `properties`, `items`, `enum`, `const`, `minimum`, `maximum`, `minLength`, and `maxLength`.
- **Reason**: The spec requires a built-in `json_schema` scorer but does not list a JSON Schema engine in Section 11.2. Implementing a useful subset in-crate keeps the default dependency surface aligned with the spec's dependency guidance while still providing a functional scorer in this iteration.
- **Downstream impact**: No internal component is blocked. Future work that depends on broader JSON Schema draft support should either extend this validator or explicitly introduce a schema-engine dependency.

### DEV-04: `Run::build()` currently requires `'static` generic types
- **Iteration**: 15
- **Component**: Run builder
- **Spec says**: Section 5.8 defines `Run::builder()...build()?` without calling out any `'static` bounds on the generic input/output/reference types or mapped output/reference types.
- **Actual**: `RunBuilderWithTargets::build()` is implemented only for `'static` `I`, `O`, `R`, and mapped output/reference types.
- **Reason**: The implementation stores the acquisition, global mapper pipeline, standalone scorers, and `ScorerSet`s behind boxed async executors so `Run<I, O, R>` can erase heterogeneous scorer pipelines while still applying global mappers exactly once per trial. That internal boxing currently requires owned `'static` generic types.
- **Downstream impact**: External callers should use owned types when constructing `Run`s. No remaining internal component is blocked because `Stats`, `Comparison`, and `JSONL` consume `RunResult` rather than building new `Run`s.

### DEV-05: Observe-mode explicit sample ID validation is heuristic
- **Iteration**: 20
- **Component**: Observe acquisition [otel]
- **Spec says**: Section 8.1 requires `RunBuildError::MissingSampleIds` for observe-mode runs that use auto-generated sample IDs.
- **Actual**: Observe-mode `Run::build()` rejects any sample ID matching the crate's stable auto-generated ID format (`[0-9a-f]{16}`) instead of tracking explicit-vs-generated provenance directly.
- **Reason**: The public `Sample` data model stores only the final `id: String`; once a sample exists, the framework cannot tell whether that value came from `Sample::new`, `SampleBuilder::build()` auto-generation, or an explicit `.id(...)` call without adding new persisted state to a completed component.
- **Downstream impact**: Observe-mode callers should use human-chosen IDs that do not look like the crate's generated hash format. If explicit hash-like IDs become a real requirement later, `Sample` will need to preserve ID provenance or expose it through a new API.

## Component Detail

### Sample
- **Status**: DONE
- **Iteration**: 2
- **Dependencies**: none
- **Files**:
  - `src/sample.rs` — defines `Sample<I, R>`, `SampleBuilder<I, R>`, deterministic auto-ID generation, and builder methods.
  - `src/lib.rs` — exports `Sample` and `SampleBuilder` from the crate root.
  - `tests/sample.rs` — 4 public API tests covering construction, deterministic IDs, explicit IDs, metadata, and builder parity with `Sample::new`.
- **AC satisfied**:
  - AC-01.1: ✅ `tests/sample.rs::sample_new_constructs_input_and_reference` verifies a `Sample<String, String>` can be constructed with input and reference values.
- **Interfaces exposed**:
  - `Sample::new(input: I, reference: R) -> Sample<I, R>` in `src/sample.rs`
  - `Sample::builder(input: I) -> SampleBuilder<I, R>` in `src/sample.rs`
  - `SampleBuilder::id(self, id: impl Into<String>) -> Self` in `src/sample.rs`
  - `SampleBuilder::reference(self, reference: R) -> Self` in `src/sample.rs`
  - `SampleBuilder::metadata(self, key: impl Into<String>, value: serde_json::Value) -> Self` in `src/sample.rs`
  - `SampleBuilder::build(self) -> Sample<I, R>` in `src/sample.rs` (see DEV-01 for bounds)
- **Deviations**: DEV-01
- **Notes for future iterations**: `Sample::new` and builder auto-ID generation use stable content hashing over input/reference only; metadata does not affect IDs, matching Section 6.1.

### Dataset
- **Status**: DONE
- **Iteration**: 3
- **Dependencies**: none
- **Files**:
  - `src/dataset.rs` — defines `Dataset<I, R>`, its empty-metadata constructor, and `From<Vec<Sample<I, R>>>` conversion.
  - `src/lib.rs` — exports `Dataset` from the crate root.
  - `tests/dataset.rs` — 2 public API tests covering direct construction and `From<Vec<Sample>>` conversion.
- **AC satisfied**:
  - AC-02.1: ✅ `tests/dataset.rs::dataset_new_constructs_from_sample_vector` and `tests/dataset.rs::dataset_from_vec_matches_new_constructor` verify `Dataset` construction from `Vec<Sample>` via both `new` and `From<Vec<Sample>>`.
- **Interfaces exposed**:
  - `Dataset::new(samples: Vec<Sample<I, R>>) -> Dataset<I, R>` in `src/dataset.rs`
  - `From<Vec<Sample<I, R>>> for Dataset<I, R>` in `src/dataset.rs`
- **Deviations**: none
- **Notes for future iterations**: `Dataset` is intentionally minimal and independent; metadata defaults to an empty map and sample order is preserved from the input vector.

### Score
- **Status**: DONE
- **Iteration**: 4
- **Dependencies**: none
- **Files**:
  - `src/score.rs` — defines the spec's `Score` enum and custom serde to preserve the required tagged JSON representation for tuple-style variants.
  - `src/lib.rs` — exports `Score` from the crate root.
  - `tests/score.rs` — 4 public API tests covering JSON serialization and round-trip deserialization for all score variants.
- **AC satisfied**:
  - None directly. `Score` is foundational for later scorer, results, stats, and comparison acceptance criteria; `tests/score.rs` verifies the Section 5.1 API and Section 6.1 tagged-JSON schema this downstream work depends on.
- **Interfaces exposed**:
  - `Score::Numeric(f64)` in `src/score.rs`
  - `Score::Binary(bool)` in `src/score.rs`
  - `Score::Label(String)` in `src/score.rs`
  - `Score::Metric { name: String, value: f64, unit: Option<String> }` in `src/score.rs`
- **Deviations**: none
- **Notes for future iterations**: `Score` intentionally does not perform eager validation. Per Sections 6.1 and 9, later scorer execution code must reject non-finite numeric values, empty labels, and invalid metrics by converting them into `ScorerError` after each scorer call.

### ScoreDefinition
- **Status**: DONE
- **Iteration**: 5
- **Dependencies**: none
- **Files**:
  - `src/score_definition.rs` — defines `ScoreDefinition`, the coupled `Direction` enum, and the spec constructors.
  - `src/lib.rs` — exports `ScoreDefinition` and `Direction` from the crate root.
  - `tests/score_definition.rs` — 4 public API tests covering constructors and JSON round-trip serialization.
- **AC satisfied**:
  - None directly. `tests/score_definition.rs` verifies the Section 5.1 constructor API and the Section 6.2 serde requirement that downstream scorer, run-result, and comparison work depends on.
- **Interfaces exposed**:
  - `ScoreDefinition { pub name: String, pub direction: Option<Direction> }` in `src/score_definition.rs`
  - `ScoreDefinition::new(name: impl Into<String>) -> ScoreDefinition` in `src/score_definition.rs`
  - `ScoreDefinition::maximize(name: impl Into<String>) -> ScoreDefinition` in `src/score_definition.rs`
  - `ScoreDefinition::minimize(name: impl Into<String>) -> ScoreDefinition` in `src/score_definition.rs`
- **Deviations**: none
- **Notes for future iterations**: `Direction` was implemented in the same module during this iteration because the exact `ScoreDefinition` API exposes `Option<Direction>` and its constructors return concrete `Direction` values.

### Direction
- **Status**: DONE
- **Iteration**: 5
- **Dependencies**: none
- **Files**:
  - `src/score_definition.rs` — defines the `Direction` enum used by `ScoreDefinition` and future comparison logic.
  - `src/lib.rs` — exports `Direction` from the crate root.
  - `tests/score_definition.rs` — constructor assertions exercise both `Direction` variants through `ScoreDefinition`.
- **AC satisfied**:
  - None directly. This is foundational for later direction-aware comparison acceptance criteria, and its presence is verified through `tests/score_definition.rs`.
- **Interfaces exposed**:
  - `Direction::Maximize` in `src/score_definition.rs`
  - `Direction::Minimize` in `src/score_definition.rs`
- **Deviations**: none
- **Notes for future iterations**: JSON serialization is currently the derived serde representation for a unit enum; no schema-level override was required by Sections 5 or 6.

### ScorerContext
- **Status**: DONE
- **Iteration**: 6
- **Dependencies**: none
- **Files**:
  - `src/scorer_context.rs` — defines the spec's `#[non_exhaustive]` `ScorerContext<'a, I, O, R = ()>` and 4 unit tests covering field access, optional references, default `R`, and generic usage.
  - `src/lib.rs` — exports `ScorerContext` from the crate root.
- **AC satisfied**:
  - None directly. `src/scorer_context.rs` unit tests verify the exact Section 5.2 shape that future scorer and run execution acceptance criteria depend on.
- **Interfaces exposed**:
  - `ScorerContext<'a, I, O, R = ()> { pub input: &'a I, pub output: &'a O, pub reference: Option<&'a R> }` in `src/scorer_context.rs`
- **Deviations**: none
- **Notes for future iterations**: `ScorerContext` is `#[non_exhaustive]` exactly as specified, so external crates cannot construct it with a struct literal; framework internals should build it inside the crate and scorers consume it by shared reference.

### ScorerError
- **Status**: DONE
- **Iteration**: 7
- **Dependencies**: none
- **Files**:
  - `src/scorer_error.rs` — defines the spec's boxed `ScorerError` wrapper plus delegated `Display` and `std::error::Error` implementations.
  - `src/lib.rs` — exports `ScorerError` from the crate root.
  - `tests/scorer_error.rs` — 4 public API tests covering wrapped display, error source propagation, and trait conformance.
- **AC satisfied**:
  - None directly. `tests/scorer_error.rs` verifies the Section 5.4 error wrapper contract and the distinct error channel required for later scorer acceptance criteria such as AC-01.5 and AC-01.6.
- **Interfaces exposed**:
  - `ScorerError(pub Box<dyn std::error::Error + Send + Sync>)` in `src/scorer_error.rs`
- **Deviations**: none
- **Notes for future iterations**: Downstream scorer and mapper code can wrap infrastructure failures without erasing the original source error; `Display` and `source()` both delegate to the boxed inner error.

### Mapper
- **Status**: DONE
- **Iteration**: 8
- **Dependencies**: none
- **Files**:
  - `src/mapper.rs` — defines the `MapError` wrapper, the `Mapper<I, O>` trait, and the closure blanket impl.
  - `src/lib.rs` — exports `Mapper` and `MapError` from the crate root.
  - `tests/mapper.rs` — 4 public API tests covering closure mapping, trait-object use, generic mapping, and `MapError` propagation.
- **AC satisfied**:
  - None directly. `tests/mapper.rs` verifies the Section 5.4 `MapError` wrapper and the Section 5.5 `Mapper` trait contract that downstream `ScorerSet` and observe-mode acceptance criteria depend on.
- **Interfaces exposed**:
  - `MapError(pub Box<dyn std::error::Error + Send + Sync>)` in `src/mapper.rs`
  - `Mapper<I, O>::map(&self, input: &I) -> Result<O, MapError>` in `src/mapper.rs`
  - `impl<F, I, O> Mapper<I, O> for F where F: Fn(&I) -> Result<O, MapError> + Send + Sync` in `src/mapper.rs`
- **Deviations**: none
- **Notes for future iterations**: `Mapper` is object-safe and closure-backed, so `ScorerSet` and OTel extraction work can store shared mappers behind trait objects if needed while still accepting plain closures from callers.

### Scorer trait
- **Status**: DONE
- **Iteration**: 9
- **Dependencies**: Score, ScorerContext, ScorerError, ScoreDefinition
- **Files**:
  - `src/scorer.rs` — defines the spec's async `Scorer<I, O, R = ()>` trait and 4 unit tests covering async scoring, error results, definition metadata, and default `R`.
  - `src/lib.rs` — exports `Scorer` from the crate root.
- **AC satisfied**:
  - None directly. `src/scorer.rs` unit tests verify the exact Section 5.2 trait surface that built-in scorers, `ScorerSet`, and run execution depend on for AC-01.2 through AC-01.6 and AC-02.6.
- **Interfaces exposed**:
  - `Scorer<I, O, R = ()>::score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError>` in `src/scorer.rs`
  - `Scorer<I, O, R = ()>::definition(&self) -> ScoreDefinition` in `src/scorer.rs`
- **Deviations**: none
- **Notes for future iterations**: The public trait matches the spec's `async fn` surface exactly and uses `#[allow(async_fn_in_trait)]` to keep the crate warning-free; downstream code should not assume `dyn Scorer` works without an explicit boxing/type-erasure strategy.

### ScorerSet
- **Status**: DONE
- **Iteration**: 10
- **Dependencies**: Scorer trait, Mapper
- **Files**:
  - `src/scorer_set.rs` — defines `ScorerSet`, its typestate builder, internal boxed executors for heterogenous async scorers, mapper error fanout, and 4 unit tests.
  - `src/lib.rs` — exports `ScorerSet` from the crate root.
- **AC satisfied**:
  - AC-06.1: ✅ `src/scorer_set.rs::tests::scorer_set_builds_with_map_output_and_scored_results` constructs a `ScorerSet` with `.map_output()` and multiple scorers.
  - AC-06.2: ✅ `src/scorer_set.rs::tests::scorer_set_builds_with_map_reference` constructs a `ScorerSet` with `.map_reference()` and a scorer consuming the transformed reference type.
  - AC-06.3: ✅ `src/scorer_set.rs::tests::output_mapper_runs_once_per_trial_and_is_shared` counts mapper invocations and proves one output mapping is reused across both scorers in the set.
  - AC-06.4: ✅ `src/scorer_set.rs::tests::mapper_errors_become_scorer_errors_for_all_scorers` verifies a mapper failure becomes `ScorerError` results for every scorer in the affected set.
- **Interfaces exposed**:
  - `ScorerSet::builder() -> ScorerSetBuilder<I, O, R>` in `src/scorer_set.rs`
  - `ScorerSetBuilder::map_output(self, mapper: M) -> ScorerSetBuilder<I, O, R, O3, R2, Mapped, ReferenceState>` in `src/scorer_set.rs`
  - `ScorerSetBuilder::map_reference(self, mapper: M) -> ScorerSetBuilder<I, O, R, O2, R3, OutputState, Mapped>` in `src/scorer_set.rs`
  - `ScorerSetBuilder::scorer(self, scorer: S) -> ScorerSetBuilderWithScorers<I, O, R, O2, R2, OutputState, ReferenceState>` in `src/scorer_set.rs`
  - `ScorerSetBuilderWithScorers::scorer(self, scorer: S) -> Self` in `src/scorer_set.rs`
  - `ScorerSetBuilderWithScorers::build(self) -> ScorerSet<I, O, R>` in `src/scorer_set.rs` (see DEV-02 for current `'static` bounds)
- **Deviations**: DEV-02
- **Notes for future iterations**: Per-set output/reference mapping runs exactly once per trial and shares mapped values across all scorers in the set. AC-06.5 remains for `Run builder`, which still needs to layer global mappers ahead of `ScorerSet` execution.

### Acquisition trait
- **Status**: DONE
- **Iteration**: 11
- **Dependencies**: none
- **Files**:
  - `src/acquisition.rs` — defines `Acquisition<I, O>`, `AcquisitionError`, their `Display`/`Error` impls, the closure blanket impl, and 4 unit tests.
  - `src/lib.rs` — exports `Acquisition` and `AcquisitionError` from the crate root.
- **AC satisfied**:
  - None directly. `src/acquisition.rs` tests verify the Section 5.6 async trait surface and closure blanket impl plus the Section 5.4 `AcquisitionError` contract required for later `Run` and observe-mode acceptance criteria such as AC-02.2 and AC-07.1.
- **Interfaces exposed**:
  - `Acquisition<I, O>::acquire(&self, input: &I) -> Result<O, AcquisitionError>` in `src/acquisition.rs`
  - `impl<I, O, F, Fut> Acquisition<I, O> for F where F: Fn(&I) -> Fut + Send + Sync, Fut: Future<Output = Result<O, AcquisitionError>> + Send` in `src/acquisition.rs`
  - `AcquisitionError::ExecutionFailed(Box<dyn std::error::Error + Send + Sync>)` in `src/acquisition.rs`
  - `AcquisitionError::TraceNotFound { correlation_id: String, sample_id: String }` in `src/acquisition.rs`
  - `AcquisitionError::BackendUnavailable(Box<dyn std::error::Error + Send + Sync>)` in `src/acquisition.rs`
  - `AcquisitionError::Timeout(Duration)` in `src/acquisition.rs`
- **Deviations**: none
- **Notes for future iterations**: `Run builder` can accept closures directly via the blanket impl without introducing an inline wrapper. `Observe acquisition` should reuse the public `TraceNotFound`, `BackendUnavailable`, and `Timeout` variants instead of creating a parallel error surface.

### Built-in scorers
- **Status**: DONE
- **Iteration**: 12
- **Dependencies**: Scorer trait
- **Files**:
  - `src/scorers/mod.rs` — defines `exact_match`, `contains`, `regex`, and `json_schema`, their internal scorer implementations, dependency-free JSON Schema subset validation, and 8 unit tests.
  - `src/lib.rs` — exposes the public `scorers` module and re-exports the built-in scorer constructor functions from the crate root.
- **AC satisfied**:
  - AC-01.2: ✅ `src/scorers/mod.rs::tests::exact_match_returns_binary_score` verifies `exact_match()` scores a referenced text output as `Score::Binary(true)`.
  - AC-01.3: ✅ `src/scorers/mod.rs::tests::contains_returns_binary_score` verifies `contains()` returns `Score::Binary(true)` when the output contains the reference substring.
  - AC-01.4: ✅ `src/scorers/mod.rs::tests::regex_returns_binary_score` verifies `regex()` produces a scorer that returns `Score::Binary(true)` for a matching output.
  - AC-01.5: ✅ `src/scorers/mod.rs::tests::missing_reference_returns_scorer_error` and `src/scorers/mod.rs::tests::json_schema_invalid_json_is_a_scorer_error` verify built-in scorers surface failures as `Err(ScorerError)` rather than bare scores.
  - AC-01.6: ✅ `src/scorers/mod.rs::tests::invalid_regex_pattern_is_distinct_from_low_score` verifies invalid regex configuration returns a construction error that is distinct from a low `Score::Binary(false)` result.
- **Interfaces exposed**:
  - `exact_match() -> impl Scorer<String, String, String>` in `src/scorers/mod.rs`
  - `contains() -> impl Scorer<String, String, String>` in `src/scorers/mod.rs`
  - `regex(pattern: &str) -> Result<impl Scorer<String, String>, regex::Error>` in `src/scorers/mod.rs`
  - `json_schema(schema: serde_json::Value) -> impl Scorer<String, String>` in `src/scorers/mod.rs`
  - `pub mod scorers` plus root re-exports for `contains`, `exact_match`, `json_schema`, and `regex` in `src/lib.rs`
- **Deviations**: DEV-03
- **Notes for future iterations**: Built-in binary scorers use `ScoreDefinition::new(...)`, so their persisted definitions carry `direction: None` as expected for binary scores. `exact_match()` and `contains()` require `Sample` references and return `ScorerError` if `reference` is absent; `regex()` and `json_schema()` work with `R = ()`.

### Run builder
- **Status**: DONE
- **Iteration**: 15
- **Dependencies**: Sample, Scorer trait, ScorerSet, Mapper, Acquisition trait
- **Files**:
  - `src/run.rs` — defines `Run`, its typestate builders, `RunBuildError`, `RunError`, boxed acquisition/scoring executors, build-time validation, global mapper execution, score validation, and async run execution.
  - `src/lib.rs` — exports `Run`, `RunBuildError`, and `RunError` from the crate root.
  - `tests/run.rs` — 5 integration tests covering dataset execution, mixed scorers and scorer sets, global mapper behavior, build validation, score validation, and sample timeouts.
- **AC satisfied**:
  - AC-02.2: ✅ `tests/run.rs::run_builder_executes_dataset_and_returns_sample_results` builds a `Run` from a dataset, an async acquisition closure, and a scorer.
  - AC-02.3: ✅ `tests/run.rs::run_builder_executes_dataset_and_returns_sample_results` verifies `Run::execute()` returns a `RunResult` with one `SampleResult` per dataset sample.
  - AC-02.4: ✅ `tests/run.rs::run_builder_executes_dataset_and_returns_sample_results` and `tests/run.rs::run_accepts_multiple_scorers_and_scorer_sets` verify each `SampleResult` contains per-trial scorer outcomes.
  - AC-02.5: ✅ All new coverage uses `#[tokio::test]` and awaits `Run::execute()`, including `tests/run.rs::run_builder_executes_dataset_and_returns_sample_results`.
  - AC-02.6: ✅ `tests/run.rs::run_accepts_multiple_scorers_and_scorer_sets` verifies one `Run` can combine standalone scorers and a `ScorerSet`.
  - AC-06.5: ✅ `tests/run.rs::global_mappers_apply_before_standalone_scorers_and_scorer_sets` verifies standalone scorers and scorer sets both receive the post-global-map output/reference values.
- **Interfaces exposed**:
  - `Run::builder() -> RunBuilder` in `src/run.rs`
  - `RunBuilder::dataset(self, dataset: impl Into<Dataset<I, R>>) -> RunBuilderWithDataset<I, R>` in `src/run.rs`
  - `RunBuilderWithDataset::acquisition(self, acquisition: A) -> RunBuilderConfigured<I, O, R>` in `src/run.rs`
  - `RunBuilderConfigured::map_output(self, mapper: M) -> RunBuilderConfigured<I, O, R, O3, R2, Mapped, ReferenceState>` in `src/run.rs`
  - `RunBuilderConfigured::map_reference(self, mapper: M) -> RunBuilderConfigured<I, O, R, O2, R3, OutputState, Mapped>` in `src/run.rs`
  - `RunBuilderConfigured::scorer(self, scorer: S) -> RunBuilderWithTargets<I, O, R, O2, R2, OutputState, ReferenceState>` in `src/run.rs`
  - `RunBuilderConfigured::scorer_set(self, scorer_set: ScorerSet<I, O2, R2>) -> RunBuilderWithTargets<I, O, R, O2, R2, OutputState, ReferenceState>` in `src/run.rs`
  - `RunBuilderWithTargets::scorer(self, scorer: S) -> Self` in `src/run.rs`
  - `RunBuilderWithTargets::scorer_set(self, scorer_set: ScorerSet<I, O2, R2>) -> Self` in `src/run.rs`
  - `RunBuilderWithTargets::trials(self, trial_count: usize) -> Self` in `src/run.rs`
  - `RunBuilderWithTargets::concurrency(self, concurrency: usize) -> Self` in `src/run.rs`
  - `RunBuilderWithTargets::sample_timeout(self, sample_timeout: Duration) -> Self` in `src/run.rs`
  - `RunBuilderWithTargets::build(self) -> Result<Run<I, O, R>, RunBuildError>` in `src/run.rs` (see DEV-04 for current `'static` bounds)
  - `Run::execute(&self) -> Result<RunResult, RunError>` in `src/run.rs`
  - `RunBuildError::{NoDataset, NoAcquisition, NoScorer, EmptyDataset, DuplicateSampleIds(Vec<String>), DuplicateScorerNames(String), MissingSampleIds}` in `src/run.rs`
  - `RunError::{Build(RunBuildError), Internal(Box<dyn std::error::Error + Send + Sync>)}` in `src/run.rs`
- **Deviations**: DEV-04
- **Notes for future iterations**: Global output/reference mappers now run exactly once per trial before both standalone scorers and scorer sets. Acquisition failures and invalid scores are surfaced as per-scorer `Err(ScorerError)` entries inside `TrialResult.scores`, so stats work should exclude all-errored trials from scored denominators rather than assuming acquisition succeeded.

### RunResult / SampleResult / TrialResult
- **Status**: DONE
- **Iteration**: 14
- **Dependencies**: Score, ScorerError, ScoreDefinition
- **Files**:
  - `src/run_result.rs` — defines `TrialResult`, `SampleResult`, `RunMetadata`, and `RunResult`, including deterministic serde for per-scorer `Result<Score, ScorerError>` entries.
  - `src/lib.rs` — exports `RunMetadata`, `RunResult`, `SampleResult`, and `TrialResult` from the crate root.
  - `tests/run_result.rs` — 4 public API tests covering score/error serialization, deserialization, score-vs-error distinction, and full `RunResult` round-tripping.
- **AC satisfied**:
  - AC-13.3: ✅ `tests/run_result.rs::sample_result_can_distinguish_low_scores_from_failed_scores` plus the serde tests verify `RunResult` stores low scores as `Ok(Score)` and scorer failures as `Err(ScorerError)` rather than collapsing them into the same representation.
- **Interfaces exposed**:
  - `TrialResult { pub scores: HashMap<String, Result<Score, ScorerError>>, pub duration: Duration, pub trial_index: usize }` in `src/run_result.rs`
  - `SampleResult { pub sample_id: String, pub trials: Vec<TrialResult>, pub trial_count: usize, pub scored_count: usize, pub error_count: usize }` in `src/run_result.rs`
  - `RunMetadata { pub run_id: String, pub started_at: DateTime<Utc>, pub completed_at: DateTime<Utc>, pub duration: Duration, pub trial_count: usize, pub score_definitions: Vec<ScoreDefinition>, pub acquisition_mode: String }` in `src/run_result.rs`
  - `RunResult { pub metadata: RunMetadata, pub samples: Vec<SampleResult> }` in `src/run_result.rs`
- **Deviations**: none
- **Notes for future iterations**: Keep raw results separate from stats per AD-10. `TrialResult.scores` serializes with sorted scorer keys and stores each outcome as either `{"Ok": <Score>}` or `{"Err": <message>}`; `Run builder` and `JSONL` should reuse that existing serde surface instead of inventing a parallel representation.

### Stats
- **Status**: DONE
- **Iteration**: 16
- **Dependencies**: RunResult / SampleResult / TrialResult, Score
- **Files**:
  - `src/stats.rs` — defines `RunStats`, `ScorerStats`, `RunResult::{stats, stats_with}`, summary formatting, and dependency-free Wilson/t-distribution confidence interval helpers.
  - `src/lib.rs` — exports `RunStats` and `ScorerStats` from the crate root.
  - `tests/stats.rs` — 5 public API tests covering numeric/metric aggregation, binary aggregation, label aggregation, error exclusion, and single-trial behavior.
- **AC satisfied**:
  - AC-10.2: ✅ `tests/stats.rs::run_result_stats_compute_numeric_and_metric_aggregates_with_t_confidence_intervals` verifies mean and standard deviation aggregation for Numeric and Metric scores.
  - AC-10.3: ✅ `tests/stats.rs::run_result_stats_compute_binary_pass_metrics_and_wilson_interval` verifies `pass_rate`, `pass_at_k`, and `pass_all_k` for Binary scores.
  - AC-10.4: ✅ `tests/stats.rs::run_result_stats_compute_label_distribution_and_mode` verifies label distribution and mode aggregation.
  - AC-10.5: ✅ `tests/stats.rs::run_result_stats_compute_numeric_and_metric_aggregates_with_t_confidence_intervals` and `tests/stats.rs::run_result_stats_compute_binary_pass_metrics_and_wilson_interval` verify t-based and Wilson confidence intervals.
  - AC-10.6: ✅ All stats tests exercise `RunResult::stats()` over multiple samples and confirm per-scorer aggregation across the whole run.
  - AC-10.7: ✅ `tests/stats.rs::run_result_stats_handle_single_trial_without_special_casing` verifies single-trial runs use the same stats structure.
  - AC-13.2: ✅ `tests/stats.rs::run_result_stats_exclude_errors_from_denominator_and_report_total_errors` verifies errored scorer outcomes are excluded from denominators and counted separately in `RunStats.total_errors`.
- **Interfaces exposed**:
  - `RunResult::stats(&self) -> RunStats` in `src/stats.rs`
  - `RunResult::stats_with(&self, confidence_level: f64) -> RunStats` in `src/stats.rs`
  - `RunStats { pub scorer_stats: HashMap<String, ScorerStats>, pub total_samples: usize, pub total_trials: usize, pub total_errors: usize }` in `src/stats.rs`
  - `RunStats::summary(&self) -> String` in `src/stats.rs`
  - `ScorerStats::{Numeric { mean: f64, stddev: f64, ci: (f64, f64), min: f64, max: f64 }, Binary { pass_rate: f64, pass_at_k: f64, pass_all_k: f64, ci: (f64, f64) }, Label { distribution: HashMap<String, usize>, mode: String }, Metric { mean: f64, stddev: f64, ci: (f64, f64), min: f64, max: f64 }}` in `src/stats.rs`
- **Deviations**: none
- **Notes for future iterations**: `RunStats.total_trials` currently reflects `RunMetadata.trial_count` (trials per sample, matching the Section 5.12 summary example) while scorer aggregation spans all samples. `RunStats.total_errors` counts errored scorer outcomes, which gives `Comparison` a ready-made infrastructure-error count without re-deriving it from raw trials.

### Comparison
- **Status**: DONE
- **Iteration**: 17
- **Dependencies**: RunResult / SampleResult / TrialResult, Stats, ScoreDefinition, Direction
- **Files**:
  - `src/comparison.rs` — defines `compare`, `CompareConfig`, `Comparison`, `ScorerComparison`, `SampleComparison`, `Change`, and dependency-free Welch/Fisher significance helpers.
  - `src/lib.rs` — exports the comparison API and public comparison types from the crate root.
  - `tests/comparison.rs` — 6 integration tests covering numeric/metric deltas, binary Fisher exact testing, configurable confidence thresholds, label comparisons, and direction mismatch handling.
- **AC satisfied**:
  - AC-03.3: ✅ `tests/comparison.rs::compare_reports_direction_aware_numeric_and_metric_sample_deltas` verifies `compare(&baseline, &candidate, ...)` produces shared-scorer per-sample deltas for two `RunResult`s.
  - AC-03.4: ✅ `tests/comparison.rs::compare_reports_direction_aware_numeric_and_metric_sample_deltas` verifies sample comparisons report `Improved`, `Regressed`, and `Unchanged`-style `Change` values instead of raw deltas alone.
  - AC-03.5: ✅ `tests/comparison.rs::compare_reports_direction_aware_numeric_and_metric_sample_deltas` verifies `Direction::Minimize` flips the meaning of improvement for latency-style metrics.
  - AC-11.1: ✅ All `tests/comparison.rs` coverage constructs baseline/candidate `RunResult`s and compares them through the public `compare(...)` API.
  - AC-11.2: ✅ `tests/comparison.rs::compare_uses_fisher_exact_for_binary_scores_with_different_trial_counts` verifies binary comparisons return a Fisher exact p-value and significance flag.
  - AC-11.3: ✅ `tests/comparison.rs::compare_uses_welch_t_test_and_marks_non_significant_deltas` verifies numeric comparisons return a Welch t-test p-value and significance flag.
  - AC-11.4: ✅ `tests/comparison.rs::compare_applies_the_configured_confidence_level_to_significance` verifies `CompareConfig.confidence_level` controls the significance threshold and defaults to `0.95`.
  - AC-11.5: ✅ `tests/comparison.rs::compare_uses_fisher_exact_for_binary_scores_with_different_trial_counts` verifies comparison works when baseline and candidate runs use different trial counts.
  - AC-11.6: ✅ `tests/comparison.rs::compare_reports_direction_aware_numeric_and_metric_sample_deltas` verifies `Maximize`/`Minimize` directions are respected when classifying changes.
  - AC-11.7: ✅ `tests/comparison.rs::compare_marks_direction_mismatch_as_incomparable` verifies mismatched baseline/candidate directions produce `Change::Incomparable`.
- **Interfaces exposed**:
  - `compare(baseline: &RunResult, candidate: &RunResult, config: CompareConfig) -> Comparison` in `src/comparison.rs`
  - `CompareConfig { pub confidence_level: f64 }` in `src/comparison.rs`
  - `Comparison { pub baseline_id: String, pub candidate_id: String, pub shared_scorers: HashMap<String, ScorerComparison>, pub only_in_baseline: Vec<String>, pub only_in_candidate: Vec<String>, pub confidence_level: f64 }` in `src/comparison.rs`
  - `ScorerComparison { pub sample_comparisons: HashMap<String, SampleComparison>, pub aggregate_delta: f64, pub p_value: Option<f64>, pub significant: Option<bool>, pub test_used: Option<String> }` in `src/comparison.rs`
  - `SampleComparison { pub sample_id: String, pub delta: f64, pub direction: Change }` in `src/comparison.rs`
  - `Change::{Improved, Regressed, Unchanged, Insignificant, Incomparable}` in `src/comparison.rs`
- **Deviations**: none
- **Notes for future iterations**: Comparison currently intersects scorer names and sample IDs across the two runs, compares scored trials only, uses Fisher's exact test for Binary scores, Welch's t-test for Numeric/Metric scores, and treats Label comparisons as change-only (`delta = 0.0`, no significance test). This resolves OQ-05 for the current implementation by allowing unequal trial counts through the same aggregate trial-level comparison path.

### JSONL
- **Status**: DONE
- **Iteration**: 18
- **Dependencies**: RunResult / SampleResult / TrialResult, serde
- **Files**:
  - `src/jsonl.rs` — defines `write_jsonl`/`read_jsonl` using a line-oriented metadata-plus-sample record format over the existing `RunResult` serde types.
  - `src/lib.rs` — exports `write_jsonl` and `read_jsonl` from the crate root.
  - `tests/jsonl.rs` — 2 integration tests covering JSONL record ordering, deterministic nested scorer encoding, and round-trip deserialization.
- **AC satisfied**:
  - AC-03.1: ✅ `tests/jsonl.rs::write_jsonl_serializes_metadata_then_samples_as_jsonl` verifies `RunResult` writes as JSONL through the public convenience function with one metadata line followed by one line per sample.
  - AC-03.2: ✅ `tests/jsonl.rs::read_jsonl_round_trips_back_to_a_typed_run_result` verifies JSONL output deserializes back into a typed `RunResult` while preserving sample order and scorer outcomes.
- **Interfaces exposed**:
  - `write_jsonl(result: &RunResult, writer: impl Write) -> Result<(), serde_json::Error>` in `src/jsonl.rs`
  - `read_jsonl(reader: impl Read) -> Result<RunResult, serde_json::Error>` in `src/jsonl.rs`
- **Deviations**: none
- **Notes for future iterations**: JSONL uses a tagged line-oriented format with one `metadata` record followed by ordered `sample` records, which keeps the output streamable while reusing `RunResult`'s existing deterministic nested serde for per-trial scores and errors.

### TraceBackend / JaegerBackend [otel]
- **Status**: DONE
- **Iteration**: 19
- **Dependencies**: none
- **Files**:
  - `src/otel.rs` — defines the `Span` and `SpanEvent` data types, `TraceBackendError`, the async `TraceBackend` trait, and the built-in HTTP/JSON `JaegerBackend` with configurable headers and retry count.
  - `src/lib.rs` — feature-gates and exports the OTel trace backend API from the crate root under `otel`.
  - `tests/otel.rs` — 3 feature-gated integration tests covering custom backends, Jaeger grouping/parsing, and retry behavior.
- **AC satisfied**:
  - AC-09.1: ✅ `tests/otel.rs::trace_backend_trait_supports_custom_implementations` verifies users can implement `TraceBackend` themselves.
  - AC-09.2: ✅ `tests/otel.rs::jaeger_backend_groups_matching_spans_by_sample_attribute` verifies the built-in `JaegerBackend` fetches and parses Jaeger HTTP/JSON traces.
  - AC-09.3: ✅ `tests/otel.rs::jaeger_backend_groups_matching_spans_by_sample_attribute` verifies fetched spans are grouped by the requested sample attribute.
  - AC-09.4: ✅ The public `TraceBackend` trait in `src/otel.rs` remains backend-agnostic, leaving future OTLP receiver work as an additional implementation rather than a contract change.
- **Interfaces exposed**:
  - `Span { pub trace_id: String, pub span_id: String, pub parent_span_id: Option<String>, pub operation_name: String, pub start_time: DateTime<Utc>, pub end_time: DateTime<Utc>, pub attributes: HashMap<String, serde_json::Value>, pub events: Vec<SpanEvent> }` in `src/otel.rs`
  - `SpanEvent { pub name: String, pub timestamp: DateTime<Utc>, pub attributes: HashMap<String, serde_json::Value> }` in `src/otel.rs`
  - `TraceBackendError(pub Box<dyn std::error::Error + Send + Sync>)` in `src/otel.rs`
  - `TraceBackend::fetch_spans(&self, correlation_id: &str, sample_attribute: &str, timeout: Duration) -> Result<HashMap<String, Vec<Span>>, TraceBackendError>` in `src/otel.rs`
  - `JaegerBackend::new(base_url: impl Into<String>) -> JaegerBackend` in `src/otel.rs`
  - `JaegerBackend::with_retry_count(self, retry_count: usize) -> JaegerBackend` in `src/otel.rs`
  - `JaegerBackend::with_header(self, name: impl AsRef<str>, value: impl AsRef<str>) -> Result<JaegerBackend, TraceBackendError>` in `src/otel.rs`
- **Deviations**: none
- **Notes for future iterations**: `JaegerBackend` currently queries Jaeger's HTTP/JSON `/api/traces` endpoint using the `eval.run_id` attribute convention from the spec and returns only spans that carry the configured `sample_attribute`. `Observe acquisition` should convert missing per-sample groups into `AcquisitionError::TraceNotFound` for expected dataset sample IDs.

### Observe acquisition [otel]
- **Status**: DONE
- **Iteration**: 20
- **Dependencies**: Acquisition trait, TraceBackend / JaegerBackend [otel]
- **Files**:
  - `src/otel.rs` — adds the feature-gated `Observe` builder/API, internal trace-backend type erasure, cached grouped span lookup, collection timeout handling, and per-sample ID task-local plumbing.
  - `src/run.rs` — marks `Run` metadata as `observe` when configured with `Observe`, validates observe-mode sample IDs, and scopes the current `Sample.id` into acquisition execution.
  - `src/lib.rs` — exports `Observe` from the crate root under the `otel` feature.
  - `tests/observe.rs` — 4 feature-gated integration tests covering observe-mode run configuration, mapper-based extraction from `Vec<Span>`, timeout behavior, missing traces, and build validation.
- **AC satisfied**:
  - AC-07.1: ✅ `tests/observe.rs::observe_mode_runs_from_grouped_spans_and_scores_like_inline_mode` configures a `Run` with `Observe` acquisition and executes it successfully.
  - AC-07.2: ✅ The same test configures `Observe::builder().correlation_id("run-abc-123")`, and its recording backend asserts that exact correlation ID is used for span collection.
  - AC-07.3: ✅ `tests/observe.rs::observe_mode_runs_from_grouped_spans_and_scores_like_inline_mode` verifies observe-mode acquisition queries a `TraceBackend` during run execution.
  - AC-07.4: ✅ `tests/observe.rs::observe_mode_runs_from_grouped_spans_and_scores_like_inline_mode` plus `tests/otel.rs::jaeger_backend_groups_matching_spans_by_sample_attribute` verify spans are grouped and matched by the configured sample attribute.
  - AC-07.5: ✅ `tests/observe.rs::observe_mode_runs_from_grouped_spans_and_scores_like_inline_mode` uses `Run::map_output(|spans: &Vec<Span>| ...)` to extract the scored output from observed spans.
  - AC-07.6: ✅ The same test proves the extracted mapper output is fed into the existing scorer pipeline and produces `Score::Binary(true)` exactly like inline mode.
  - AC-07.7: ✅ `tests/observe.rs::observe_mode_uses_collection_timeout_for_backend_fetches` verifies observe-mode timeout handling, and `tests/otel.rs::jaeger_backend_retries_failed_requests_before_returning` covers configurable backend retries.
  - AC-07.8: ✅ `tests/observe.rs::observe_mode_maps_missing_sample_spans_to_trace_not_found` verifies missing spans become `AcquisitionError::TraceNotFound` scorer errors rather than low scores.
  - AC-07.9: ✅ `Observe` is defined/exported only under the `otel` feature and all observe integration coverage is in `#![cfg(feature = "otel")]` tests run via `cargo test --features otel`.
  - AC-07.10: ✅ The observe API exposes a single `Observe::builder()...build()` path with no separate replay mode; `tests/observe.rs::observe_mode_runs_from_grouped_spans_and_scores_like_inline_mode` exercises the same acquisition path used for repeated trial reads.
- **Interfaces exposed**:
  - `Observe::builder() -> ObserveBuilder` in `src/otel.rs`
  - `ObserveBuilder::backend(self, backend: B) -> ObserveBuilderWithBackend` in `src/otel.rs`
  - `ObserveBuilderWithBackend::correlation_id(self, correlation_id: impl Into<String>) -> ObserveBuilderWithCorrelationId` in `src/otel.rs`
  - `ObserveBuilderWithCorrelationId::sample_attribute(self, sample_attribute: impl Into<String>) -> ObserveBuilderWithSampleAttribute` in `src/otel.rs`
  - `ObserveBuilderWithSampleAttribute::timeout(self, timeout: Duration) -> ObserveBuilderReady` in `src/otel.rs`
  - `ObserveBuilderReady::build(self) -> Observe` in `src/otel.rs`
  - `impl<I> Acquisition<I, Vec<Span>> for Observe` in `src/otel.rs`
- **Deviations**: DEV-05
- **Notes for future iterations**: Observe-mode acquisition caches grouped spans per `Observe` instance after the first backend fetch, so repeated trials for the same correlation ID reuse the same fetched span set. The implementation relies on `Run` to supply the current `Sample.id` via an internal task-local context because the public `Acquisition::acquire(&self, input: &I)` signature does not carry sample IDs.

### LLM-as-a-Judge scorer [llm-judge]
- **Status**: DONE
- **Dependencies**: Scorer trait
- **Iteration**: 21
- **Files**:
  - `src/scorers/mod.rs` — adds the feature-gated `llm_judge` scorer, `LlmJudgeConfig`, extraction strategies, OpenAI-compatible chat-completions request/response handling, prompt rendering, and `ScorerError` mapping for network/response/parse failures.
  - `src/lib.rs` — feature-gates and exports `llm_judge`, `LlmJudgeConfig`, and `LlmJudgeScoreExtractor` from the crate root.
  - `tests/llm_judge.rs` — 5 feature-gated integration tests covering rendered prompt contents, score parsing, optional references, network/parse error handling, and API-key serde skipping.
- **AC satisfied**:
  - AC-05.1: ✅ `tests/llm_judge.rs::llm_judge_sends_prompt_and_parses_score_json` configures the scorer with a prompt template and `LlmJudgeScoreExtractor::JsonScore`, and `tests/llm_judge.rs::llm_judge_supports_boolean_extraction_without_reference` configures `LlmJudgeScoreExtractor::Boolean`.
  - AC-05.2: ✅ `tests/llm_judge.rs::llm_judge_sends_prompt_and_parses_score_json` asserts the outbound request contains rendered `input`, `output`, and `reference` values and verifies the response content is parsed into `Score::Numeric(0.75)`.
  - AC-05.3: ✅ `tests/llm_judge.rs::llm_judge_network_errors_return_scorer_error` and `tests/llm_judge.rs::llm_judge_parse_errors_return_scorer_error` verify transport and response-parsing failures surface as `ScorerError` entries rather than low scores.
  - AC-05.4: ✅ All new scorer coverage is async `#[tokio::test]` integration coverage executing the scorer through `Run`, including `tests/llm_judge.rs::llm_judge_sends_prompt_and_parses_score_json`.
  - AC-05.5: ✅ `llm_judge`, `LlmJudgeConfig`, and `LlmJudgeScoreExtractor` are exported only behind `#[cfg(feature = "llm-judge")]` in `src/lib.rs`, and the scorer implementation plus integration tests are feature-gated in `src/scorers/mod.rs` and `tests/llm_judge.rs`.
- **Interfaces exposed**:
  - `llm_judge(config: LlmJudgeConfig) -> impl Scorer<String, String, String>` in `src/scorers/mod.rs`
  - `LlmJudgeConfig { pub model: String, pub base_url: String, pub prompt_template: String, pub score_extractor: LlmJudgeScoreExtractor, pub api_key: String }` in `src/scorers/mod.rs`
  - `LlmJudgeScoreExtractor::{JsonScore, Boolean, Numeric, Label}` in `src/scorers/mod.rs`
- **Deviations**: none
- **Notes for future iterations**: Prompt rendering substitutes `{{input}}`, `{{output}}`, and `{{reference}}` directly into one user message. `LlmJudgeConfig.api_key` is `#[serde(skip)]` so configs serialize without secrets, and the current extractor strategies parse either tagged `Score` JSON or simple boolean/numeric/label text responses.

## Blocked Items

None.

## Build Log

| Iteration | Component | Status | Duration | Notes |
|-----------|-----------|--------|----------|-------|
| 1 | Bootstrap | DONE | - | Initialized the `evalkit` library crate, configured spec dependencies and feature flags, created `src/scorers`, `src/otel`, and `tests`, replaced Cargo sample code with an empty crate root, and verified with `cargo test` (0 passed, 0 failed). |
| 2 | Sample | DONE | - | Implemented `Sample` and `SampleBuilder`, exported them from the crate root, added 4 tests, and verified with `cargo test`. |
| 3 | Dataset | DONE | - | Implemented `Dataset`, exported it from the crate root, added 2 tests for `new` and `From<Vec<Sample>>`, and verified with `cargo test`. |
| 4 | Score | DONE | - | Implemented the `Score` enum with spec-compliant tagged JSON serialization, exported it from the crate root, added 4 tests, and verified with `cargo test`. |
| 5 | ScoreDefinition | DONE | - | Implemented `ScoreDefinition` and the coupled `Direction` enum, exported both from the crate root, added 4 constructor/serde tests, and verified with `cargo test`. |
| 6 | ScorerContext | DONE | - | Implemented the `#[non_exhaustive]` `ScorerContext` struct, exported it from the crate root, added 4 unit tests, and verified with `cargo test`. |
| 7 | ScorerError | DONE | - | Implemented the boxed `ScorerError` wrapper, exported it from the crate root, added 4 tests for display/source/trait behavior, and verified with `cargo test`. |
| 8 | Mapper | DONE | - | Implemented `Mapper` and `MapError`, exported both from the crate root, added 4 tests for closure blanket impl and error propagation, and verified with `cargo test`. |
| 9 | Scorer trait | DONE | - | Implemented the async `Scorer` trait, exported it from the crate root, added 4 unit tests for scoring, error results, definition metadata, and default `R`, and verified with `cargo test`. |
| 10 | ScorerSet | DONE | - | Implemented `ScorerSet` with typestate builder methods for optional per-set output/reference mappers, shared per-trial mapping across heterogeneous scorers, exported it from the crate root, added 4 unit tests, and verified with `cargo fmt && cargo test`. |
| 11 | Acquisition trait | DONE | - | Implemented `Acquisition` plus `AcquisitionError`, exported both from the crate root, added 4 unit tests for custom impls, closure blanket impl support, and error propagation/display, and verified with `cargo fmt && cargo test`. |
| 12 | Built-in scorers | DONE | - | Implemented `exact_match`, `contains`, `regex`, and `json_schema`, exported them from the crate root and `scorers` module, added 8 unit tests for AC-01.2 through AC-01.6 plus JSON-schema behavior, and verified with `cargo fmt && cargo test`. |
| 13 | Run builder | BLOCKED | - | Re-read the Run builder and result-type spec sections, confirmed the current crate still passes `cargo test` (47 passed, 0 failed), and recorded that `Run` cannot be completed before `RunResult / SampleResult / TrialResult` because Section 5.8 already exposes those public result types. |
| 14 | RunResult / SampleResult / TrialResult | DONE | - | Implemented the raw run result structs plus `RunMetadata`, exported them from the crate root, added 4 tests for score/error serde and `RunResult` round-tripping, verified with `cargo fmt && cargo test`, and cleared the `Run builder` ordering blocker. |
| 15 | Run builder | DONE | - | Implemented `Run` with typestate builder ordering, async acquisition, global mapper execution, mixed standalone scorer and `ScorerSet` support, build-time validation, score validation, and sample timeouts; added 5 integration tests and verified with `cargo fmt && cargo test` (52 passed, 0 failed). |
| 16 | Stats | DONE | - | Implemented `RunStats`/`ScorerStats`, `RunResult::{stats, stats_with}`, dependency-free Wilson/t-based confidence intervals, and 5 stats tests; verified with `cargo fmt && cargo test` (57 passed, 0 failed). |
| 17 | Comparison | DONE | - | Implemented the comparison API and public comparison types, added dependency-free Fisher exact and Welch t significance testing, covered direction-aware and unequal-trial comparisons with 6 integration tests, and verified with `cargo fmt && cargo test` (63 passed, 0 failed). |
| 18 | JSONL | DONE | - | Implemented `write_jsonl`/`read_jsonl` as line-oriented metadata-plus-sample helpers, exported them from the crate root, added 2 integration tests, and verified with `cargo fmt && cargo test` (65 passed, 0 failed). |
| 19 | TraceBackend / JaegerBackend [otel] | DONE | - | Implemented the feature-gated `Span`/`SpanEvent` types, `TraceBackend` trait, and built-in HTTP/JSON `JaegerBackend` with header and retry configuration, added 3 `otel` integration tests, and verified with `cargo fmt && cargo test && cargo test --features otel` (68 passed, 0 failed). |
| 20 | Observe acquisition [otel] | DONE | - | Implemented the feature-gated `Observe` builder/acquisition, integrated observe-mode sample-ID scoping and metadata into `Run`, added 4 `otel` integration tests for mapper-based extraction, missing traces, timeout handling, and build validation, and verified with `cargo fmt && cargo test && cargo test --features otel` (72 passed, 0 failed). |
| 21 | LLM-as-a-Judge scorer [llm-judge] | DONE | - | Implemented the feature-gated `llm_judge` scorer plus `LlmJudgeConfig` and extractor strategies, added 5 `llm-judge` integration tests for prompt rendering, response parsing, error handling, and secret-safe serde, and verified with `cargo fmt && cargo test && cargo test --features llm-judge && cargo test --features otel && cargo test --features "otel llm-judge"` (77 passed, 0 failed under `llm-judge`). |
