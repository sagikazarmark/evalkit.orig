# Evalkit PR Comment Action

This composite action runs `evalkit run`, diffs the fresh run against a baseline JSONL with `evalkit diff`, and posts the markdown diff to the current pull request.

Example workflow:

```yaml
name: Evalkit

on:
  pull_request:

jobs:
  evalkit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: ./.github/actions/evalkit-pr-comment
        with:
          dataset: path/to/dataset.jsonl
          config: path/to/eval.toml
          baseline: path/to/baseline.jsonl
```

Inputs:
- `dataset`: dataset JSONL path
- `config`: evalkit TOML config path
- `baseline`: baseline run JSONL path for `evalkit diff`
- `working-directory`: Cargo workspace root, default `.`
- `rust-toolchain`: Rust toolchain, default `stable`
- `results-output`: output JSONL path for the fresh run
- `diff-output`: markdown diff path
- `json-output`: JSON diff path
- `github-token`: token used to create or update the pull-request comment
