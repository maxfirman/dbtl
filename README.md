# dbtl

`dbtl` is a Rust CLI for exploring dbt model lineage from `manifest.json` and printing a readable ASCII DAG in the terminal.

It is designed for fast local inspection, with short commands and selector syntax that is close to dbt's `--select` lineage operators.

## What It Does

- Reads dbt metadata from `<target-path>/manifest.json` (default target path: `target`)
- Builds a model-only DAG (`resource_type == "model"`)
- Renders lineage as left-to-right ASCII graphs
- Supports selector unions and depth-limited traversal

## Installation

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
```

### Arguments

- `--target-path <DIR>`: directory containing `manifest.json` (default: `target`)
- `-s, --select <SELECTOR>...`: one or more selectors (space-separated) to union

If `--select` is omitted, `dbtl` renders all model nodes in the manifest.

## Selector Syntax

Supported selector forms:

- `model` - selected node only
- `model+` - selected node + all descendants
- `+model` - all ancestors + selected node
- `+model+` - ancestors + selected node + descendants
- `N+model` - ancestors up to depth `N` + selected node
- `model+N` - selected node + descendants up to depth `N`
- `N+model+M` - ancestors up to `N` and descendants up to `M`

Depth values must be positive integers (`>= 1`).

### Union selectors

Multiple selectors are unioned into one rendered subgraph:

```bash
dbtl -s model_a model_b+
dbtl -s 1+orders stg_customers
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

### Example output shape

```text
[stg_orders]---------+--------------------+
                     |                    |
[stg_order_items]----+-->[order_items]----+-->[orders]----+-->[customers]
[stg_products]-------+                                    |
[stg_supplies]-------+                                    |
[stg_customers]-------------------------------------------+
```

## Behavior Notes

- Node type scope is currently **models only**.
- Model name resolution uses `nodes.*.name`.
- Ambiguous names (same model name in multiple packages) return an error with candidates.
- Disconnected components are rendered as separate blocks.

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
