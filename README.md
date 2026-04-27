# reposnap

Recursively scan a directory for git repositories and emit per-repo metadata as JSON / CSV / TSV / Markdown / ASCII table.

## Install

```sh
# Homebrew (macOS / Linux)
brew install misebox/tap/reposnap

# crates.io
cargo install reposnap

# From source
cargo build --release   # → ./target/release/reposnap
```

## Runtime dependencies

Called as subprocesses. Missing tools are auto-detected and the
corresponding fields are skipped silently. Run `reposnap --diagnose`
for a status table and per-OS install hints.

| Tool    | Required | Used for |
|---------|----------|----------|
| `git`   | yes      | all repository scanning |
| `gh`    | no       | GitHub description, is_private, open issues, open PRs |
| `docker`| no       | compose runtime status (`docker compose ps`) |

LOC counting is built in (uses the `tokei` library directly).

## Usage

```sh
reposnap [ROOT_DIR] [OPTIONS]
```

| Option | Default | Purpose |
|---|---|---|
| `--format <FMT>` | `ascii` | One of: `ascii`, `json`, `csv`, `tsv`, `markdown` |
| `--output <PATH>` | – | Write to file instead of stdout |
| `--fields <LIST>` | all | Pick output fields (see below). Group aliases: `@minimal`, `@activity`, `@github` |
| `--filter <EXPR>` | – | Row filter (see Filtering). Repeatable, ANDed |
| `--only-dirty` | off | Sugar for `--filter has_uncommitted=true` |
| `--only-unpushed` | off | Sugar for `--filter unpushed_commits>0` |
| `--only-tag <T>` | – | Sugar for `--filter tech_tags~T` (repeatable, AND) |
| `--sort <FIELDS>` | path | Sort by field(s); `-` prefix = desc; comma separated |
| `--limit <N>` | – | Truncate to first N rows after sort/filter |
| `--quiet` / `-q` | off | Suppress tracing log output (errors only) |
| `--no-github` | off | Skip `gh` lookups |
| `--no-docker` | off | Skip `docker compose ps` |
| `--no-loc` | off | Skip line counting |
| `--include-nested` | off | Descend into nested git repos |
| `--exclude <NAME>` | – | Extra dir name to skip (repeatable) |
| `--concurrency <N>` | `4` | Repositories scanned in parallel |
| `--diagnose` | off | Print external tool status and install hints, then exit |

### Examples

```sh
reposnap ~/repos/private                                       # ascii table to stdout
reposnap ~/repos/private --format json --output catalog.json   # JSON to file
reposnap ~/repos/private --format markdown > catalog.md        # markdown via shell redirect
reposnap --diagnose                                            # check which tools are available
reposnap ~/repos/private --filter "ahead>0" --fields path,branch,ahead
reposnap ~/repos/private --sort=-last_commit_date --limit 10
```

## Fields

Canonical field names (used in `--fields`, `--filter`, `--sort`, and CSV/TSV/JSON/Markdown headers):

```
path  name  current_branch
github_repo  is_private  github_description
last_commit_hash  last_commit_date  last_commit_message
has_uncommitted  uncommitted_files  uncommitted_insertions  uncommitted_deletions
unpushed_commits  unmerged_branches  open_issues  open_prs
loc  scale  dir_size_bytes
has_readme  has_dockerfile  compose_file  compose_ports  compose_running
tech_tags
```

### ASCII column ↔ canonical name (aliases)

The ASCII table uses short headers. Their lowercase form is accepted as an alias
in `--fields`, `--filter`, and `--sort`:

| ASCII | alias | canonical |
|---|---|---|
| BRANCH | `branch` | `current_branch` |
| LAST COMMIT DATE | – | `last_commit_date` |
| DIRTY | `dirty` | `has_uncommitted` |
| AHEAD | `ahead` | `unpushed_commits` |
| UNMERGED | `unmerged` | `unmerged_branches` |
| GH | `gh` | `github_repo` |
| PRIV | `priv` | `is_private` |
| DESC | `desc` | `github_description` |
| TAGS | `tags` | `tech_tags` |
| README | `readme` | `has_readme` |
| SIZE | `size` | `dir_size_bytes` |
| DOCKERFILE | `dockerfile` | `has_dockerfile` |
| COMPOSE | `compose` | `compose_file` |
| PORTS | `ports` | `compose_ports` |
| RUNNING | `running` | `compose_running` |

## Filtering

`--filter <expr>` selects rows whose field matches a condition. The expression
is `field<op>value`. Repeat the flag for AND.

### Operators

| op | meaning | applicable types |
|----|---------|------------------|
| `=`, `!=` | equal / not equal | string, bool, number, scale |
| `>`, `<`, `>=`, `<=` | order | number, scale, ISO date string |
| `~` | substring / array contains | string, `tech_tags`, `compose_ports` |

### Type semantics

- **string**: full match for `=`/`!=`, substring for `~`. Date strings (ISO 8601) compare lexically, which matches chronological order.
- **bool**: accepts `true`/`false`/`yes`/`no`/`1`/`0` (case-insensitive).
- **number**: integer comparison.
- **scale**: ordered as `small < medium < large < huge` (logical, not lexical).
- **arrays** (`tech_tags`, `compose_ports`): `~ value` checks "contains".

### Null handling

When a field has no value (e.g. `unpushed_commits` with no upstream),
**no condition matches**. Such rows are dropped from filtered output.

### Examples

```sh
reposnap --filter "ahead>0"
reposnap --filter "last_commit_date<2025-01-01"
reposnap --filter "scale>=large"
reposnap --filter "tech_tags~rust" --filter "tech_tags~wasm"
reposnap --filter "compose_ports~8080"
reposnap --filter "github_description~hello, world"   # value may contain commas
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
