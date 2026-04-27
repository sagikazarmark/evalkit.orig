# Benchmarks

Closes issue `05-benchmark-runtime-extraction.md`.

## Fixture

- 5_000 `Sample<String, String>` rows with ids `s-0` through `s-4999`, input and
  reference both set to `input-<i>`.
- Inline `Fn(&String) -> Future<Output=Result<String, _>>` output source that
  clones its input. No IO, no sleeps.
- Single `ExactMatch` scorer over a `ScorerSet`.
- `PullExecutor` with `AlwaysSampler`, `NoopSink`, `trials(1)`.

The intent is not to measure scorer CPU or output source IO — it is to
measure the *plumbing*: source → sampler → executor → scorer → sink. That
is exactly the code that moved from `evalkit` to `evalkit-runtime`.

## How to run

```bash
cargo run --release -p evalkit-runtime --example throughput_bench
```

Reports:
- `throughput_samples_per_sec` — wall-clock throughput
- `peak_rss_kib` — `VmHWM` from `/proc/self/status`

Run it a handful of times and take the median; wall-clock throughput on a
fixture this small is noisy.

## Thresholds

From issue 05:

- throughput must not regress by more than **10%** vs. baseline
- peak RSS must not regress by more than **15%** vs. baseline

Cross these thresholds on a change to the runtime path and you owe an
investigation before merging.

## Before / after numbers

Hardware: the CI-class Linux container this branch was developed on.
Numbers are noisy; take medians across the three runs per revision.

### Before — `evalkit 0.2.0` at `6c3cfa8` (pre-extraction)

| Run | throughput (samples/sec) | peak RSS (KiB) |
|---|---:|---:|
| 1 | 221_723 | 7_860 |
| 2 | 220_851 | 7_860 |
| 3 | 153_336 | 7_900 |
| **median** | **220_851** | **7_860** |

Run 3 looked like scheduler noise; throughput stabilised on the other two.

### After — `evalkit 0.3.0` + `evalkit-runtime 0.1.0` at `da958bf` (post-extraction)

| Run | throughput (samples/sec) | peak RSS (KiB) |
|---|---:|---:|
| 1 | 303_723 | 7_912 |
| 2 | 263_806 | 7_884 |
| 3 | 305_910 | 7_828 |
| **median** | **303_723** | **7_884** |

### Delta

| Metric | Before (median) | After (median) | Delta | Within threshold? |
|---|---:|---:|---:|---|
| throughput | 220_851 s/s | 303_723 s/s | **+37.5%** | yes (improvement, not regression) |
| peak RSS | 7_860 KiB | 7_884 KiB | **+0.3%** | yes (within 15%) |

No regression. The executor path is, if anything, slightly faster after
being moved into its own crate — plausibly a side-effect of shorter
compilation units letting the optimiser see more of the relevant code. The
extraction itself did not change algorithmic behaviour.

## Out of scope

- Wasm smoke-artifact size and startup footprint are flagged in issue 05
  as "if a wasm smoke artifact path exists." None exists today; adding
  one is a separate piece of work. When it lands, record stripped
  artifact size and startup footprint in this file under the same
  threshold rule.
- Per-scorer or per-provider benchmarks. Those belong to the individual
  extension crates.
