# The `Eval` facade

`Eval` is the recommended quickstart API for `evalkit`. It is additive: it
does not replace, deprecate, or rename any of the existing kernel types.
Internally it compiles down to the same `Run::builder()` path, so score
definitions, result shape, and `RunMetadata` semantics are identical.

## Use `Eval` when

- You are writing your first eval and want the shortest working chain.
- The eval is a single acquisition paired with one or more scorers.
- You don't need output or reference mappers.
- You want `samples → acquire → score → run` to be one expression.

## Use `Run::builder()` when

- You need `map_output` or `map_reference` to transform into a different
  type before scoring.
- You want to attach a `ScorerSet` rather than individual scorers.
- You need custom `ScorerResources` accounting or judge-model pinning
  paths beyond `judge_model_pin`.
- You want to inspect build errors separately from execution errors.
- You want to construct the `Run` now and execute it later.

The facade is a thin wrapper. Anything it can express, `Run::builder()` can
also express with one or two more lines. Anything `Run::builder()` can
express, the facade can opt out of via `EvalRun::into_run()`.

## Equivalence

See `src/eval.rs::tests::facade_produces_same_shape_as_kernel_path`. For a
fixed dataset, acquisition, trial count, and seed, the facade and the raw
`Run::builder()` path produce:

- identical `score_definitions` in `RunMetadata`
- identical per-sample / per-trial score outcomes
- identical trial counts and seed
- identical `acquisition_mode`

The facade is not a new execution engine — it's a different shape for the
same one.

## Quickstart shape

```rust
use evalkit::prelude::*;

let result = Eval::new(samples)
    .acquire(acquisition)
    .scorer(MyScorer)
    .trials(3)
    .run()
    .await?;
```

See `examples/quickstart.rs` for a runnable version.
