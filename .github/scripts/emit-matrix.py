#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Emit the wheel build matrix as JSON for the `release.yml` and
`warm.yml` plan jobs to publish.

Reads `platforms.toml`, keeps the entries that carry a `runner` (every
wheel row does, the sdist row does not), drops the fields no matrix row
consumes (`label`, `pattern`, `smoke`), and writes `matrix=<json>` to
`$GITHUB_OUTPUT` for the wheel-building jobs to read via
`fromJSON(needs.plan.outputs.matrix)`.

Also writes `smoke=wheels-<name>`, naming the artifact `release.yml`'s
validate job installs, taken from the row flagged `smoke`.
"""

from json    import dumps
from os      import environ
from pathlib import Path
from tomllib import loads


if __name__ == "__main__":

    platforms = loads(Path(".github/scripts/platforms.toml").read_text())["platforms"]
    include   = [
        {k: v for k, v in p.items() if k not in {"label", "pattern", "smoke"}}
        for p in platforms if "runner" in p
    ]
    smoke     = next(p["name"] for p in platforms if p.get("smoke"))

    with open(environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
        f.write(f"matrix={dumps({'include': include})}\n")
        f.write(f"smoke=wheels-{smoke}\n")
