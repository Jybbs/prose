#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Audit cross-config version pins for drift.

Reads each pair's two sources, normalizes per the pair's rule, and
exits 0 when every pair agrees. Mismatches surface as `::error::`
annotations naming the file pair and the divergent values.

Initial pairs:
    Rust version    `README.md` badge vs `crate/Cargo.toml` `rust-version`
    Python version  `README.md` badge vs `crate/pyproject.toml` `requires-python`
"""

from pathlib import Path
from re      import search
from tomllib import loads


def badge(svg: str) -> str:
    """
    Return the `<major>.<minor>` token from the README badge line whose
    link target carries `svg`.
    """
    for line in Path("README.md").read_text(encoding="utf-8").splitlines():
        if svg in line and (match := search(r"(\d+\.\d+)\+", line)):
            return match.group(1)
    raise SystemExit(f"::error::no README.md badge line carries {svg!r}")


def major_minor(value: str) -> str:
    """
    Return `<major>.<minor>` from any string carrying a SemVer head.
    """
    if match := search(r"\d+\.\d+", value):
        return match.group(0)
    raise SystemExit(f"::error::cannot parse major.minor from {value!r}")


if __name__ == "__main__":

    cargo   = loads(Path("crate/Cargo.toml").read_text(encoding="utf-8"))
    project = loads(Path("crate/pyproject.toml").read_text(encoding="utf-8"))

    pairs = [
        (
            "README.md Rust badge ↔ crate/Cargo.toml rust-version",
            badge("rust.svg"),
            major_minor(cargo["package"]["rust-version"])
        ),
        (
            "README.md Python badge ↔ crate/pyproject.toml requires-python",
            badge("python.svg"),
            major_minor(project["project"]["requires-python"])
        )
    ]

    failed = 0
    for label, left, right in pairs:
        if left != right:
            print(f"::error::parity mismatch in {label}: {left!r} vs {right!r}")
            failed = 1

    raise SystemExit(failed)
