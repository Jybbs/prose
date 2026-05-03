#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Validate that pyproject.toml and Cargo.toml agree with the pushed tag.

Reads from the environment:
    GITHUB_REF_TYPE  e.g. tag, branch
    GITHUB_REF_NAME  e.g. 0.1.0, main

Exits 0 on a non-tag run, or on a tag whose declared pyproject.toml and
Cargo.toml versions match. Exits 1 when either file disagrees with the tag.
"""

from os      import environ
from re      import sub
from tomllib import load


if environ.get("GITHUB_REF_TYPE") != "tag":
    raise SystemExit

version = environ["GITHUB_REF_NAME"]
for file, table, expected in [
    ("pyproject.toml", "project", version),
    ("Cargo.toml",     "package", sub(r"\.?(a|b|rc|dev|post)(\d+)", r"-\1.\2", version)),
]:
    with open(file, "rb") as f:
        if (actual := load(f)[table]["version"]) != expected:
            raise SystemExit(f"::error::{file} version mismatch: expected {expected}, got {actual}")
