#!/usr/bin/env bash
# Bump version, run checks, tag, and push.
# Usage:
#   scripts/release.sh patch | minor | major | --current
# Options:
#   --dry-run    Print plan without making changes
#   -y           Skip confirmation prompt
#   --no-push    Stop after creating the tag locally
set -euo pipefail

cd "$(dirname "$0")/.."

dry_run=0
yes=0
no_push=0
mode=""

while [ $# -gt 0 ]; do
  case "$1" in
    patch|minor|major|--current) mode="$1" ;;
    --dry-run) dry_run=1 ;;
    -y) yes=1 ;;
    --no-push) no_push=1 ;;
    -h|--help)
      sed -n '2,9p' "$0"
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
  shift
done

[ -n "$mode" ] || { echo "specify one of: patch | minor | major | --current" >&2; exit 2; }

run() {
  if [ "$dry_run" -eq 1 ]; then
    echo "DRY: $*"
  else
    eval "$@"
  fi
}

current_version() {
  awk '
    /^\[package\]/    { in_pkg = 1; next }
    /^\[/             { in_pkg = 0 }
    in_pkg && /^version[[:space:]]*=/ {
      gsub(/[" ]/, "")
      sub(/^version=/, "")
      print
      exit
    }
  ' Cargo.toml
}

next_version() {
  local cur="$1" kind="$2"
  local base="${cur%%-*}"
  IFS='.' read -r ma mi pa <<<"$base"
  case "$kind" in
    patch) pa=$((pa + 1)) ;;
    minor) mi=$((mi + 1)); pa=0 ;;
    major) ma=$((ma + 1)); mi=0; pa=0 ;;
    *) echo "$cur"; return ;;
  esac
  echo "${ma}.${mi}.${pa}"
}

write_version() {
  local v="$1"
  awk -v v="$v" '
    BEGIN { done = 0 }
    /^\[package\]/    { in_pkg = 1; print; next }
    /^\[/             { in_pkg = 0 }
    in_pkg && !done && /^version[[:space:]]*=/ {
      print "version = \"" v "\""
      done = 1
      next
    }
    { print }
  ' Cargo.toml > Cargo.toml.new
  mv Cargo.toml.new Cargo.toml
}

cur=$(current_version)
[ -n "$cur" ] || { echo "could not read version from Cargo.toml" >&2; exit 1; }

if [ "$mode" = "--current" ]; then
  new="$cur"
else
  new=$(next_version "$cur" "$mode")
fi
tag="v${new}"

# Sanity checks
branch=$(git rev-parse --abbrev-ref HEAD)
[ "$branch" = "main" ] || { echo "must be on main (got: $branch)" >&2; exit 1; }
if ! git diff-index --quiet HEAD --; then
  echo "working tree is not clean" >&2
  exit 1
fi
git fetch --tags origin main >/dev/null 2>&1 || true
local_head=$(git rev-parse HEAD)
remote_head=$(git rev-parse origin/main 2>/dev/null || true)
if [ -n "$remote_head" ] && [ "$local_head" != "$remote_head" ]; then
  echo "local main is not in sync with origin/main" >&2
  exit 1
fi
if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "tag $tag already exists" >&2
  exit 1
fi

echo "current : $cur"
echo "next    : $new"
echo "tag     : $tag"
echo "mode    : $mode"
[ "$dry_run" -eq 1 ] && echo "(dry run)"
if [ "$yes" -ne 1 ] && [ "$dry_run" -ne 1 ]; then
  printf "proceed? [y/N] "
  read -r ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "aborted"; exit 1; }
fi

run "cargo fmt --check"
run "cargo test --quiet"

if [ "$mode" != "--current" ]; then
  if [ "$dry_run" -eq 1 ]; then
    echo "DRY: bump Cargo.toml version $cur -> $new"
  else
    write_version "$new"
    cargo update -p reposcan --precise "$new" >/dev/null 2>&1 || true
    git add Cargo.toml Cargo.lock
    git commit -m "Release ${tag}"
  fi
fi

run "git tag -a \"$tag\" -m \"Release ${tag}\""

if [ "$no_push" -eq 1 ]; then
  echo "skipped push (--no-push). To publish: git push origin main \"$tag\""
  exit 0
fi

run "git push origin main \"$tag\""

cat <<EOF

Release tag ${tag} pushed.
Next steps:
  - GitHub Actions builds binaries and updates the Homebrew formula.
  - Publish to crates.io manually:
      cargo publish --dry-run
      cargo publish
    NOTE: crates.io is immutable. A version cannot be re-uploaded once published.
EOF
