#!/usr/bin/env python3
"""Print the workspace's publishable crates in dependency order.

`cargo publish` uploads one crate at a time, and a crate cannot resolve
until everything it names is already on the registry. The order is therefore
part of the release, and a hand-written list of it goes stale the first time
a crate is added: the workspace grew from three members to seven while the
release workflow still named three, so a release would have stopped at the
first crate whose dependency had never been uploaded.

The order comes from `cargo metadata`, so adding a crate is enough.

Usage:
    scripts/publish-order.py            # one crate per line, dependencies first
    scripts/publish-order.py --json     # the same list as JSON
"""

from __future__ import annotations

import json
import subprocess
import sys


def workspace_members() -> tuple[dict[str, set[str]], dict[str, str]]:
    """Internal dependency edges and versions, keyed by crate name."""
    raw = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    meta = json.loads(raw)

    packages = {p["name"]: p for p in meta["packages"]}
    names = set(packages)

    edges: dict[str, set[str]] = {}
    versions: dict[str, str] = {}
    for name, pkg in packages.items():
        # `publish = false` opts a crate out; `publish = ["registry"]` limits
        # it. Either way an empty list means "nowhere", which is the only
        # case that removes a crate from the release.
        if pkg.get("publish") == []:
            continue
        versions[name] = pkg["version"]
        edges[name] = {
            dep["name"]
            for dep in pkg["dependencies"]
            # A dev-dependency is not part of what a consumer resolves, so it
            # does not constrain the upload order.
            if dep["name"] in names and dep["kind"] != "dev"
        }

    # Drop edges to crates that are not being published at all.
    for name in edges:
        edges[name] &= set(edges)
    return edges, versions


def topological(edges: dict[str, set[str]]) -> list[str]:
    """Dependencies first, ties broken by name so the order is reproducible."""
    ordered: list[str] = []
    placed: set[str] = set()
    remaining = dict(edges)

    while remaining:
        ready = sorted(
            name for name, deps in remaining.items() if deps <= placed
        )
        if not ready:
            cycle = ", ".join(sorted(remaining))
            raise SystemExit(f"dependency cycle among: {cycle}")
        for name in ready:
            ordered.append(name)
            placed.add(name)
            del remaining[name]
    return ordered


def main() -> int:
    edges, versions = workspace_members()
    order = topological(edges)
    if "--json" in sys.argv:
        print(json.dumps(order))
    elif "--with-versions" in sys.argv:
        for name in order:
            print(f"{name} {versions[name]}")
    else:
        print("\n".join(order))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
