#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Validate that `crate/Cargo.toml` agrees with the pushed tag.

Reads from the environment:
    `GITHUB_REF_TYPE`  e.g. tag, branch
    `GITHUB_REF_NAME`  e.g. 0.1.0, main

Exits 0 on a non-tag run, or on a tag whose `crate/Cargo.toml` version matches.
Exits 1 when `crate/Cargo.toml` disagrees with the tag.
"""

from os      import environ
from pathlib import Path
from tomllib import loads


if __name__ == "__main__":

    if environ.get("GITHUB_REF_TYPE") != "tag":
        raise SystemExit

    expected = environ["GITHUB_REF_NAME"]
    actual   = loads(Path("crate/Cargo.toml").read_text())["package"]["version"]

    if actual != expected:
        raise SystemExit(
            f"::error::crate/Cargo.toml version mismatch: expected {expected}, got {actual}"
        )
