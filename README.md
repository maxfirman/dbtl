# dbtl

`dbtl` is a Rust CLI for exploring dbt model lineage from `manifest.json` and printing a readable ASCII DAG in the terminal.

It is designed for fast local inspection, with short commands and selector syntax that is close to dbt's `--select` lineage operators.

## What It Does

- Reads dbt metadata from `<target-path>/manifest.json` (default target path: `target`)
- Builds a model-only DAG (`resource_type == "model"`)
- Renders lineage as left-to-right ASCII graphs
- Supports dbt-style set operators, methods, and graph traversal selectors

## Installation

### Install from GitHub release binary

Download the latest Linux release archive and extract `dbtl`:

```bash
curl -fL -o dbtl.tar.gz \
  https://github.com/maxfirman/dbtl/releases/latest/download/dbtl-v0.1.3-x86_64-unknown-linux-gnu.tar.gz
tar -xzf dbtl.tar.gz
chmod +x dbtl
```

Optional: move it onto your `PATH`:

```bash
sudo mv dbtl /usr/local/bin/dbtl
```

After installation, you can update to the newest GitHub release in place:

```bash
dbtl self update
```

### Build from source

```bash
cargo build --release
```

Binary path:

```bash
./target/release/dbtl
```

### Run without installing

```bash
cargo run -- --target-path /path/to/target -s +orders+
```

## Usage

```bash
dbtl [--target-path <DIR>] [-s|--select <SELECTOR>...]
dbtl self update
```

### Arguments

- `--target-path <DIR>`: directory containing `manifest.json` (default: `target`)
- `-s, --select <SELECTOR>...`: one or more selectors (space-separated union)

If `--select` is omitted, `dbtl` renders all model nodes in the manifest.

## Selector Syntax

Set operators:

- Space-separated selector arguments are **unioned**.
- Comma-separated terms inside one selector argument are **intersected**.

Supported selector forms:

- `model` - selected node only
- `model+` - selected node + all descendants
- `+model` - all ancestors + selected node
- `+model+` - ancestors + selected node + descendants
- `N+model` - ancestors up to depth `N` + selected node
- `model+N` - selected node + descendants up to depth `N`
- `@model` - model + descendants + ancestors required by descendants
- `N+model+M` - ancestors up to `N` and descendants up to `M`

Supported selector methods:

- `tag:<value>` (supports `*` and `?` wildcards)
- `fqn:<value>` (supports `*` and `?` wildcards)
- `path:<value>` (supports exact file, directory prefix, and wildcards)
- `config.<key.path>:<value>` (supports nested keys and scalar/array matches)

Depth values must be positive integers (`>= 1`).

### Set operator examples

Space-separated selectors are unioned:

```bash
dbtl -s model_a model_b+
dbtl -s 1+orders stg_customers
```

Comma-separated selectors are intersected:

```bash
dbtl -s tag:finance,config.materialized:table
dbtl -s fqn:pkg.marts.*,path:models/marts
```

## Examples

### Show full DAG from default `target/manifest.json`

```bash
dbtl
```

### Show model plus all descendants

```bash
dbtl -s orders+
```

### Show one-hop ancestors only

```bash
dbtl -s 1+customers
```

### Show bounded both directions

```bash
dbtl -s 2+orders+1
```

### Show buildable closure with `@`

```bash
dbtl -s @orders
```

### Example output shape

```text
[stg_orders]---------•--------------------+
                     |                    |
                     |                    |
[stg_order_items]----•                    |
                     |                    |
                     •-->[order_items]----•-->[orders]----•-->[customers]
                     |                                    |
[stg_products]-------•                                    |
                     |                                    |
[stg_supplies]-------+                                    |
[stg_customers]-------------------------------------------+
```

## Behavior Notes

- Node type scope is currently **models only**.
- Bare selectors first resolve exact model names via `nodes.*.name`.
- Ambiguous names (same model name in multiple packages) return an error with candidates.
- Disconnected components are rendered as separate blocks.
- State-based selector methods are intentionally not implemented yet.

## Exit Codes

- `0`: success
- `1`: runtime/data error (missing manifest, model not found, ambiguous model, parse failure)
- `2`: usage/selector validation error

## Development

### Run tests

```bash
cargo test
```

### Lint (treat warnings as errors)

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Coverage

```bash
cargo llvm-cov --workspace --all-features --summary-only
```

## Limitations / Future Work

- No package-qualified selector input yet (for directly resolving ambiguous names)
- No non-model node rendering (tests/sources/seeds/etc.)
- Current renderer is terminal-optimized ASCII (not SVG/interactive)
