# Session Notes

## 2026-04-23

### Current roadmap status

- Phases 0 through 5 now have concrete in-repo implementations for the currently intended scope.
- Remaining roadmap work is mostly deferred or hardening-oriented rather than missing critical-path features.

### Explicitly deferred items

- Live GitHub Action validation against a real pull request environment
- Networked Phase 2 sources such as Kafka or NATS

### Major server UX landed in this session

- Filterable review queue with quick triage actions
- Stored output playback for runs, including intermediate snapshots
- Side-by-side sample adjudication in diffs with baseline/candidate playback and per-scorer change summaries

### Phase 5 status

Landed:

- Deterministic sharding
- PII scrubbing hooks
- Drift detection
- Heuristic red-team scorer pack

Open follow-on work is hardening rather than missing core Phase 5 scope.

### Release recommendation from this session

- `0.1.0`: go
- `1.0.0`: no-go for the whole workspace

Reasoning:

- The project is strong enough for a feature-complete preview release.
- The kernel is more mature than the full workspace.
- Newer runtime, server, and governance surfaces still need more burn-in before a whole-workspace `1.0.0` promise would be comfortable.

### Operational note

Disk pressure is recurrent during Rust builds and tests.

Working pattern that succeeded:

- clear `target/debug` frequently
- prefer exact focused tests over broad test runs
- use `RUSTFLAGS="-C debuginfo=0"`
- use `CARGO_INCREMENTAL=0` when test binaries get too large

### Useful recent commits

- `a252937` `feat: add server review queue`
- `8d15af0` `feat: add stored run output playback`
- `3667496` `feat: add diff adjudication view`

### Best next product slices

- threaded collaboration in `evalkit-server`
- auth / multi-user review workflow
- background review jobs

For broader roadmap depth beyond current scope:

- live GitHub Action validation
- deferred networked sources

### Kernel boundary follow-up from this session

- The real problem is not "remove Tokio." The real problem is shrinking the semver-critical root crate.
- `docs/stability.md` is part of the blast radius. Moving root exports is also a release-contract decision.
- `docs/evalkit-kernel-boundary-plan.md` is now the current source of truth for this topic.
- The issue order matters: do the boundary audit before runtime extraction.
- The slippery decisions are still `Run`, JSONL helpers, and the conversation / trajectory sample shapes.
- `wasm32` compile success alone is not the whole portability story. Dependency surface and portability budgets matter.
- The additive facade needs equivalence tests, or the project will drift into two APIs.
- Packaging and naming were deferred into `TODOS.md`, not resolved.

### Related issues opened

- `#3` Audit the evalkit root crate boundary
- `#4` Create `evalkit-runtime` and move runtime-oriented APIs out of the root crate
- `#5` Add a semver-safe happy-path facade for first-use DX
- `#6` Add contract tests that prove the kernel boundary change is real
- `#7` Benchmark the runtime extraction so the production path does not regress
