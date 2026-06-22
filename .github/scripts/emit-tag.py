#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Emit the release tag for `draft.yml` to consume.

Reads `[package].version` from `crate/Cargo.toml` at HEAD and HEAD~1, writes
`version=<tag>` and `changed=<true|false>` to `$GITHUB_OUTPUT`. The `changed`
flag holds when the two versions differ, counting a HEAD~1 that lacks the
manifest or carries no parseable version as changed.
"""

from os         import environ
from pathlib    import Path
from subprocess import run
from tomllib    import TOMLDecodeError, loads


def previous_version() -> str | None:
    """
    Return `[package].version` from `crate/Cargo.toml` at HEAD~1, or `None`
    when HEAD~1 lacks the manifest or carries no parseable version.
    """
    show = run(
        ["git", "show", "HEAD~1:crate/Cargo.toml"],
        capture_output = True,
        text           = True
    )
    if show.returncode != 0:
        return None
    try:
        return loads(show.stdout)["package"]["version"]
    except (KeyError, TOMLDecodeError):
        return None


if __name__ == "__main__":

    head = loads(Path("crate/Cargo.toml").read_text())["package"]["version"]
    prev = previous_version()

    with open(environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
        f.write(f"version={head}\n")
        f.write(f"changed={'true' if prev != head else 'false'}\n")
