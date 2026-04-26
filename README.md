# reposcan

Recursively scan a directory for git repositories and emit per-repo metadata as JSON / CSV / TSV / Markdown / ASCII table.

## Install

```sh
# Homebrew (macOS / Linux)
brew install misebox/tap/reposcan

# crates.io
cargo install reposcan

# From source
cargo build --release   # → ./target/release/reposcan
```

## Runtime dependencies

Called as subprocesses. Missing tools are auto-detected and the
corresponding fields are skipped silently. Run `reposcan --diagnose`
for a status table and per-OS install hints.

| Tool    | Required | Used for |
|---------|----------|----------|
| `git`   | yes      | all repository scanning |
| `gh`    | no       | GitHub description, is_private, open issues, open PRs |
| `docker`| no       | compose runtime status (`docker compose ps`) |

LOC counting is built in (uses the `tokei` library directly).

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
| `--diagnose` | off | Print external tool status and install hints, then exit |

### Examples

```sh
reposcan ~/repos/private                                       # ascii table to stdout
reposcan ~/repos/private --format json --output catalog.json   # JSON to file
reposcan ~/repos/private --format markdown > catalog.md        # markdown via shell redirect
reposcan --diagnose                                            # check which tools are available
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
`null` and the scan continues.
