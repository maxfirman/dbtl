# AGENTS.md

Guidance for AI agents working in this repository.

## Project Summary

`dbtl` is a Rust CLI that reads a dbt `manifest.json`, builds a model-only dependency graph, and renders lineage as ASCII.

Core capabilities currently include:
- dbt-style selector set operators
  - union: space-separated selector arguments
  - intersection: comma-separated terms within one selector argument
- selector methods (non-state)
  - `tag:`
  - `fqn:`
  - `path:`
  - `config.<key.path>:`
- graph operators
  - `+` with optional depth
  - `@` with dbt-compatible constraints (`@...+` is invalid)

## Important Scope / Behavior

- Selection scope is **models only**. Do not include tests/sources/seeds in selection output.
- Keep `@` behavior aligned with dbt model selection behavior.
- Keep parser behavior aligned with dbt where practical; when intentionally divergent, document it in README and tests.
- State-based selector methods are intentionally out of scope for now.

## Repository Map

- `src/main.rs`: CLI entrypoint and top-level flow
- `src/cli.rs`: clap argument parsing only
- `src/selector.rs`: selector parsing + evaluation semantics
- `src/manifest.rs`: manifest deserialization types
- `src/graph.rs`: model graph index + traversal + metadata matching helpers
- `src/render.rs`: render orchestration and component handling
- `src/render/layout.rs`: layout algorithm
- `src/render/ascii.rs`: ASCII canvas rendering details
- `tests/cli.rs`: integration tests for CLI selectors/output
- `tests/dbt_jaffle.rs`: optional dbt-backed integration test
- `tests/fixtures/`: fixture manifest and dbt project

## Required Validation Before Finishing

Run all of the following:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

If output snapshots change, update snapshot files intentionally and verify diffs.

## dbt Integration Test Notes

`tests/dbt_jaffle.rs` is gated by env var:

- `DBTL_RUN_DBT_ITEST=1` to enable

CI has a dedicated job for this path that installs dbt.

## Rendering Rules to Preserve

- Junction glyph semantics:
  - `•` for join/diverge (T-junction) points
  - `+` for corners and crossing/join intersections
- Maintain enforced vertical spacing between stacked junction glyphs.
- Arrow stems should keep `--` before `>`.

If you change renderer logic, ensure all renderer unit tests and CLI snapshots are updated and still readable.

## Selector/Parser Change Guidelines

When touching selector behavior:
- Add/update unit tests in `src/selector.rs` for parser + evaluator behavior.
- Add/update CLI tests in `tests/cli.rs` for end-to-end behavior.
- If dbt parity is claimed, verify against `dbt ls` with `--resource-type model` on fixture project.

## Release / Distribution

Release workflow is tag-driven (`v*`) and publishes Linux tarball assets.

When changing install docs:
- Keep README installation instructions consistent with actual release asset naming.

## Safe Editing Preferences

- Keep changes minimal and local to the feature.
- Avoid broad refactors unless required by behavior/maintainability.
- Prefer explicit error messages and stable test expectations.
- Preserve CLI compatibility unless a behavior change is explicitly requested.

## Commit Hygiene

- Group related changes into logical commits.
- Include tests with behavior changes.
- Keep README in sync with user-visible behavior.
