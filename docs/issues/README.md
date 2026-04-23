# Issue Drafts

These files are GitHub-ready issue drafts derived from `docs/evalkit-kernel-boundary-plan.md`.

Open them in this order:

1. `01-audit-root-crate-boundary.md`
2. `02-create-evalkit-runtime-and-move-runtime-apis.md`
3. `03-add-semver-safe-happy-path-facade.md`
4. `04-add-boundary-contract-tests.md`
5. `05-benchmark-runtime-extraction.md`

Dependency shape:

```text
01 boundary audit
   |
   +--> 02 runtime extraction
   |
   +--> 03 additive facade
            |
            +--> 04 contract tests
02 runtime extraction --+
                         +--> 05 benchmark
```

Notes:

- `01` is the forcing function. Do not skip it.
- `02` and `03` can overlap a little, but they both touch `src/` and should probably stay in one code lane.
- `04` should land in the same milestone as the code it validates.
- `05` is small, but it should use a real before/after baseline instead of vibes.
