# Root Crate Dependency Surface: Before / After

Before/after record of the `evalkit` root crate's external dependency surface,
as the runtime extraction lands. Required by issue 04 so reviewers can see
what the boundary change bought us at the crate-manifest level.

## Before (`evalkit` 0.2.0, pre-extraction)

```toml
[dependencies]
chrono      = { version = "0.4", features = ["serde"] }
futures     = { version = "0.3", default-features = false, features = ["std", "async-await"] }
regex       = "1"
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
tokio       = { version = "1", features = ["macros", "rt", "time"] }
uuid        = { version = "1", features = ["serde", "v4"] }
```

Seven direct dependencies. The `regex` entry was only pulled in by the PII
scrubber path in the old `executor` module. `tokio` was pulled in by both
the kernel (task_local, timeout) and the old executor (sleep, timeout).

## After (`evalkit` 0.3.0, post-extraction)

```toml
[dependencies]
chrono      = { version = "0.4", features = ["serde"] }
futures     = { version = "0.3", default-features = false, features = ["std", "async-await"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
tokio       = { version = "1", features = ["macros", "rt", "time"] }
uuid        = { version = "1", features = ["serde", "v4"] }
```

Six direct dependencies. `regex` is gone from the kernel. All runtime-only
dependency baggage moved to `evalkit-runtime` along with the code.

## Delta

| Dep | Before | After | Moved to |
|---|---|---|---|
| `regex` | root | (dropped) | `evalkit-runtime` |
| `chrono` | root | root | also in `evalkit-runtime` |
| `futures` | root | root | also in `evalkit-runtime` |
| `tokio` | root, `["macros","rt","time"]` | root, `["macros","rt","time"]` | also in `evalkit-runtime` |
| `uuid` | root | root | also in `evalkit-runtime` |
| `serde` | root | root | also in `evalkit-runtime` |
| `serde_json` | root | root | also in `evalkit-runtime` |

## What this means for portability

The root crate is now closer to a batch-kernel posture: no regex, no file
tailing, no sharding. The `tokio` feature set is still `macros + rt + time`
because `Run` still uses `tokio::time::timeout` and `acquisition` uses
`task_local`. That is intentional — timeout handling on `Run` is explicitly
flagged as a follow-up in `docs/root-crate-boundary-audit.md`.

The CI wasm check (see `.github/workflows/boundary.yml`) guards against
accidental regressions to the portability story.
