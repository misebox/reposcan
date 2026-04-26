# reposcan

Recursively scan a directory for git repositories and emit per-repo metadata as JSON / CSV / TSV / Markdown / ASCII table.

## Build

```sh
cargo build --release
# Binary: ./target/release/reposcan
```

Runtime dependencies (called as subprocesses, optional unless noted):

- `git` (required)
- `gh` (for GitHub metadata)
- `docker` (for compose runtime status)
- `tokei` (for fast LOC counting; falls back to `git ls-files` + reading)

## Usage

```sh
reposcan [ROOT_DIR] [OPTIONS]
```

| Option | Default | Purpose |
|---|---|---|
| `--format <FMT>` | `ascii` | One of: `ascii`, `json`, `csv`, `tsv`, `markdown` |
| `--output <PATH>` | – | Write to file instead of stdout |
| `--no-github` | off | Skip `gh` lookups |
| `--no-docker` | off | Skip `docker compose ps` |
| `--no-loc` | off | Skip line counting |
| `--include-nested` | off | Descend into nested git repos |
| `--exclude <NAME>` | – | Extra dir name to skip (repeatable) |
| `--concurrency <N>` | `4` | Repositories scanned in parallel |

### Examples

```sh
reposcan ~/repos/private                                       # ascii table to stdout
reposcan ~/repos/private --format json --output catalog.json   # JSON to file
reposcan ~/repos/private --format markdown > catalog.md        # markdown via shell redirect
```

## Pruned directories

The walker never descends into:

```
node_modules .git dist build .next .nuxt target vendor
__pycache__ .venv venv .cache .turbo .vercel coverage out
```

When a `.git` directory or file is detected, that subtree is treated as one
repository and the walker stops descending unless `--include-nested` is set.

## Output schema

See `src/types.rs`. All collection steps are non-fatal: a failed lookup yields
`null` and a `tracing::warn` line on stderr. The whole scan keeps going.
