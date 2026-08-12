#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

for path in README.md CHANGELOG.md CONTRIBUTING.md SECURITY.md CITATION.cff LICENSE; do
  test -s "$path" || {
    echo "missing release document: $path" >&2
    exit 1
  }
done

grep -q '^## Ecosystem$' README.md
grep -q 'https://github.com/HaoZeke/another tool' README.md
grep -q 'another tool' CHANGELOG.md
grep -q 'another tool' CITATION.cff

for manifest in crates/vissue-core/Cargo.toml crates/vissue-cli/Cargo.toml crates/vissue-mcp/Cargo.toml; do
  grep -q '^description = ' "$manifest"
  grep -q '^readme.workspace = true$' "$manifest"
done

echo "vissue release surface: ok"
