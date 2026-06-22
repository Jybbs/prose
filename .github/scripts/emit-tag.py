#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Emit the release tag for `draft.yml` to consume.

Reads `[package].version` from `crate/Cargo.toml` at HEAD and HEAD~1, writes
`version=<tag>` and `changed=<true|false>` to `$GITHUB_OUTPUT`. The
`changed` flag lets push-triggered runs short-circuit when the bump
didn't actually land.
"""

from os         import environ
from pathlib    import Path
from subprocess import run
from tomllib    import loads


if __name__ == "__main__":

    head = loads(Path("crate/Cargo.toml").read_text())["package"]["version"]
    prev = loads(run(
        ["git", "show", "HEAD~1:crate/Cargo.toml"],
        capture_output = True,
        check          = True,
        text           = True
    ).stdout)["package"]["version"]

    with open(environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
        f.write(f"version={head}\n")
        f.write(f"changed={'true' if head != prev else 'false'}\n")
