#!/usr/bin/env bash
# Regenerate crates/vissue-core/src/schema/vissue_capnp.rs from vissue.capnp.
#
# One command where both tools are present, which is the normal case. Where they
# are not, it says which is missing and what to do instead, because the two halves
# can live on different machines and the request file moves between them.
#
# Nothing in the build runs this. The generated file is committed, and
# `the_generated_constant_matches_the_schema_text` fails if it is stale, so
# forgetting to run this is caught by the suite rather than shipped.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
out="$here/../crates/vissue-core/src/schema/vissue_capnp.rs"

have() { command -v "$1" >/dev/null 2>&1; }

if ! have capnp; then
  echo "capnp is not on PATH: it compiles the schema." >&2
  echo "Install Cap'n Proto, or run this on a machine that has it." >&2
  exit 1
fi

if have capnpc-rust; then
  # Into a scratch directory, and then found rather than assumed. The plugin lays
  # its output out by the source path recorded in the request, so it can land in a
  # `schema/` subdirectory rather than flat, and a `mv` of a guessed name fails on
  # whichever of the two it did not guess.
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  capnp compile -orust:"$tmp" "$here/vissue.capnp"
  generated="$(find "$tmp" -name 'vissue_capnp.rs' -print -quit)"
  if [ -z "$generated" ]; then
    echo "the plugin wrote no vissue_capnp.rs under $tmp" >&2
    exit 1
  fi
  mv "$generated" "$out"
  echo "regenerated $out"
  exit 0
fi

# Only the compiler is here. Emit the request so the plugin can run elsewhere.
request="${TMPDIR:-/tmp}/vissue-capnp-request.bin"
capnp compile -o- "$here/vissue.capnp" > "$request"
cat >&2 <<EOF
capnpc-rust is not on PATH: it turns the schema into Rust.

Wrote the compiler's request to:
  $request

On a machine with the plugin:
  capnpc-rust < $request
and put the resulting schema/vissue_capnp.rs at:
  crates/vissue-core/src/schema/vissue_capnp.rs

Or install the plugin here with: cargo install capnpc
EOF
exit 1
