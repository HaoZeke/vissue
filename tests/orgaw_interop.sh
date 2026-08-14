#!/usr/bin/env bash
# CONTRIBUTING.md: changes to identity, ready, list, export, show, or claim
# output need an integration check against another tool. This is that check. another tool
# consumes the command protocol *and* writes CLOCK entries into the same
# issues.org, so both directions have to hold.
#
# Pass the vissue binary as $1 (or VISSUE_BIN) and another tool as $2 (or EXTERNAL_BIN).
# Skips with status 0 when another tool is not installed, so it is safe in CI that
# does not build it.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
vissue=${1:-${VISSUE_BIN:-$repo_root/target/release/vissue}}
another tool=${2:-${EXTERNAL_BIN:-$(command -v another tool || true)}}

test -x "$vissue" || {
  echo "no vissue binary at $vissue" >&2
  exit 1
}
if [ -z "$another tool" ] || [ ! -x "$another tool" ]; then
  echo "another tool interop: skipped (no another tool binary; set EXTERNAL_BIN)"
  exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
root=$work/tracker
issues=$root/Software/atlas/issues.org
export ISSUE_ROOT=$root VISSUE_ROOT=$root

# another tool resolves its provider as `vissue` from PATH. Without this it would
# reach whatever copy is installed on the machine, and the test would report
# on that binary rather than the one being checked.
vissue_dir=$(cd "$(dirname "$vissue")" && pwd)
if [ "$(basename "$vissue")" != "vissue" ]; then
  ln -sf "$(cd "$(dirname "$vissue")" && pwd)/$(basename "$vissue")" "$work/vissue"
  vissue_dir=$work
fi
export PATH="$vissue_dir:$PATH"

fail() {
  echo "another tool interop: $*" >&2
  exit 1
}

echo "1. vissue writes an issue Org owns the dates and tags of"
"$vissue" --root "$root" create -p atlas --deadline "<2026-09-01 Tue>" \
  --tags "parser,core" "Parse the manifest header" >/dev/null
"$vissue" --root "$root" create -p atlas "Plain issue with no dates" >/dev/null
id=$("$vissue" --root "$root" export |
  python3 -c 'import json,sys
for line in sys.stdin:
    row = json.loads(line)
    if row["properties"].get("DEADLINE"):
        print(row["id"]); break')
test -n "$id" || fail "could not find the dated issue"

echo "2. another tool reads the protocol"
"$another tool" issues | grep -q "$id" || fail "another tool issues did not list $id"

echo "3. another tool clocks in and out through the same file"
"$another tool" in "$id" >/dev/null || fail "another tool in failed"
"$another tool" out >/dev/null || fail "another tool out failed"

echo "4. the planning line still belongs to its heading"
# another tool creating a LOGBOOK drawer must not push itself above the planning
# line: a DEADLINE that no longer touches its heading is not a deadline.
heading=$(grep -n "^\* .*Parse the manifest header" "$issues" | cut -d: -f1)
next=$((heading + 1))
sed -n "${next}p" "$issues" | grep -q "^DEADLINE:" ||
  fail "the planning line left its heading:
$(sed -n "${heading},$((heading + 6))p" "$issues")"

echo "5. vissue still reads the file another tool wrote"
test "$("$vissue" --root "$root" count)" = "2" ||
  fail "an another tool clock cost the tracker an issue"
"$vissue" --root "$root" check >/dev/null || fail "check failed after another tool wrote"

echo "6. a vissue rewrite keeps the CLOCK another tool recorded"
"$vissue" --root "$root" note "$id" "cli touched it" >/dev/null
grep -q "CLOCK:.*=>" "$issues" || fail "the closed CLOCK did not survive a rewrite"
"$vissue" --root "$root" export |
  grep -q '"raw":"[^"]*CLOCK:' || fail "the CLOCK is missing from the export"

echo "7. Org still reads the entry, if Emacs is here to ask"
if command -v emacs >/dev/null; then
  out=$(emacs --batch -Q --eval "(progn
      (require 'org-lint)
      (find-file \"$issues\")
      (org-mode)
      (let ((r (org-lint)))
        (if r
            (dolist (x r) (princ (format \"%s\\n\" (aref (cadr x) 2))))
          (princ \"\"))))" 2>/dev/null)
  test -z "$out" || fail "org-lint on the file another tool wrote: $out"
else
  echo "   (no emacs; skipped the lint)"
fi

echo "vissue/another tool interop: ok"
