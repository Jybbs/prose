#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Emit the wheel build matrix as JSON for `release.yml` to consume.

Reads `platforms.toml`, keeps the entries that carry a `runner` (every
wheel row does, the sdist row does not), drops the summary-only fields
(`label`, `pattern`), and writes `matrix=<json>` to `$GITHUB_OUTPUT`
for the `build` job to read via `fromJSON(needs.plan.outputs.matrix)`.
"""

from json    import dumps
from os      import environ
from pathlib import Path
from tomllib import loads


if __name__ == "__main__":

    here      = Path(__file__).parent
    platforms = loads((here / "platforms.toml").read_text())["platforms"]
    include   = [
        {k: v for k, v in p.items() if k not in {"label", "pattern"}}
        for p in platforms if "runner" in p
    ]

    with open(environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
        f.write(f"matrix={dumps({'include': include})}\n")
