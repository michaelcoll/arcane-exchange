#!/usr/bin/env bash
# Print the dependency versions that moved between a base ref and the working tree.
#
# Usage, from the repo root: bash .agents/skills/bump-deps/scripts/version-list.sh [base-ref]
# (base-ref defaults to origin/main). Direct dependencies are listed as `name: old → new`, transitive ones as a count.
set -euo pipefail

base=${1:-origin/main}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# Writes a file both as it is at the base ref (old) and in the working tree (new).
snapshot() { git show "$base:$1" >"$tmp/old"; cp "$1" "$tmp/new"; }

# compare <title> <old> <new> <direct> [extra-lines]: the inputs are `name version` lines; direct, one name per line.
compare() {
  echo "## $1"
  awk -v direct="$4" '
    BEGIN { while ((getline name < direct) > 0) d[name] = 1 }
    FNR == 1 { file++ }
    { v[file, $1] = (v[file, $1] == "" ? "" : v[file, $1] ", ") $2; names[$1] = 1 }
    END {
      for (n in names) if (v[1, n] != v[2, n]) {
        if (n in d) printf "- %s: %s → %s\n", n, (v[1, n] == "" ? "∅" : v[1, n]), (v[2, n] == "" ? "∅" : v[2, n])
        else transitive++
      }
      printf "+ %d\n", transitive
    }
  ' <(sort -V "$2") <(sort -V "$3") >"$tmp/out"
  { grep '^- ' "$tmp/out" | sort; [[ -n ${5:-} ]] && echo "$5"; } >"$tmp/direct" || true
  if [[ -s $tmp/direct ]]; then cat "$tmp/direct"; else echo "- no direct dependency"; fi
  echo "- $(grep '^+ ' "$tmp/out") transitive"
  echo
}

# Cargo
cargo_versions() { awk -F' = ' '/^\[\[package\]\]/ { n = "" } $1 == "name" { n = $2 } $1 == "version" { print n, $2 }' "$1" | tr -d '"'; }
awk '/^\[/ { s = $0 } s ~ /^\[(dev-|build-)?dependencies\]$/ && /^[A-Za-z0-9_-]+[ .=]/ { sub(/[ .=].*/, ""); print }' \
  backend/Cargo.toml >"$tmp/cargo-direct"
snapshot backend/Cargo.lock
cargo_versions "$tmp/old" >"$tmp/cargo-old"
cargo_versions "$tmp/new" >"$tmp/cargo-new"
compare Cargo "$tmp/cargo-old" "$tmp/cargo-new" "$tmp/cargo-direct"

# pnpm: the lockfile holds two YAML documents (pnpm itself, then the project); only the last one is the project.
pnpm_project() { awk '/^---$/ { buf = ""; next } { buf = buf $0 "\n" } END { printf "%s", buf }' "$1"; }
pnpm_section() { pnpm_project "$1" | awk -v want="$2:" '/^[^ ]/ { s = $0 } s == want'; }
pnpm_versions() { pnpm_section "$1" packages | sed -nE "s/^  '?(@?[^@' ]+)@([^' (]+)'?:$/\1 \2/p"; }
snapshot frontend/pnpm-lock.yaml
pnpm_versions "$tmp/old" >"$tmp/pnpm-old"
pnpm_versions "$tmp/new" >"$tmp/pnpm-new"
pnpm_section "$tmp/new" importers | sed -nE "s/^      '?([^' ]+)'?:$/\1/p" >"$tmp/pnpm-direct"
snapshot frontend/.nuxtrc
nuxtrc=$(paste -d'|' <(comm -23 <(sort "$tmp/old") <(sort "$tmp/new")) <(comm -13 <(sort "$tmp/old") <(sort "$tmp/new")) |
  sed -E 's/^(.*)\|(.*)$/- .nuxtrc: \1 → \2/')
compare pnpm "$tmp/pnpm-old" "$tmp/pnpm-new" "$tmp/pnpm-direct" "$nuxtrc"

# SwiftPM: Package.resolved pins the whole graph; the direct packages are the ones APIClient/Package.swift names.
swift_versions() { jq -r '.pins[] | "\(.identity) \(.state.version // .state.revision[0:8])"' "$1"; }
grep -oE 'url: "[^"]+"' ios-app/APIClient/Package.swift | sed -E 's#.*/([^/"]+)"$#\1#; s/\.git$//' |
  tr '[:upper:]' '[:lower:]' >"$tmp/swift-direct"
snapshot ios-app/APIClient/Package.resolved
swift_versions "$tmp/old" >"$tmp/swift-old"
swift_versions "$tmp/new" >"$tmp/swift-new"
compare SwiftPM "$tmp/swift-old" "$tmp/swift-new" "$tmp/swift-direct"
