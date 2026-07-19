#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Audit cross-config version pins for drift.

Reads each pair's two sources, normalizes per the pair's rule, and exits 0
when every pair agrees. Mismatches surface as `::error::` annotations naming
the file pair and the divergent values.
"""

from pathlib import Path
from re      import search
from tomllib import loads


def action_pin(action: str, key: str) -> str:
    """
    Return the `key` input pinned in the `action` composite action.
    """
    return extract(
        f"no {key} pin in {action}/action.yml",
        rf"{key}\s*:\s*v?(\S+)",
        Path(f".github/actions/{action}/action.yml").read_text(encoding="utf-8")
    )


def badge(svg: str) -> str:
    """
    Return the `<major>.<minor>` token from the README badge line whose link
    target carries `svg`.
    """
    return extract(
        f"no README.md badge line carries {svg!r}",
        rf"{svg}.*?(\d+\.\d+)\+",
        Path("README.md").read_text(encoding="utf-8")
    )


def build_requirement(name: str) -> str:
    """
    Return the exact version `crate/pyproject.toml` pins for the `name`
    build requirement.
    """
    for requirement in toml("crate/pyproject.toml")["build-system"]["requires"]:
        if requirement.startswith(name):
            return extract(
                f"{name} build requirement is not an exact pin",
                r"==\s*(\S+)",
                requirement
            )
    raise SystemExit(f"::error::no build-system requirement named {name!r}")


def cargo_lock_version(name: str) -> str:
    """
    Return the version `Cargo.lock` resolves for the `name` package.
    """
    for package in toml("Cargo.lock")["package"]:
        if package["name"] == name:
            return package["version"]
    raise SystemExit(f"::error::no Cargo.lock package named {name!r}")


def extract(error: str, pattern: str, text: str) -> str:
    """
    Return the first capture of `pattern` in `text`.
    """
    if match := search(pattern, text):
        return match.group(1)
    raise SystemExit(f"::error::{error}")


def major_minor(value: str) -> str:
    """
    Return `<major>.<minor>` from any string carrying a SemVer head.
    """
    return extract(f"cannot parse major.minor from {value!r}", r"(\d+\.\d+)", value)


def toml(path: str) -> dict:
    """
    Return the parsed TOML mapping at `path`.
    """
    return loads(Path(path).read_text(encoding="utf-8"))


if __name__ == "__main__":

    cargo   = toml("Cargo.toml")
    mise    = toml(".mise/config.toml")
    project = toml("crate/pyproject.toml")
    RUST    = cargo["workspace"]["package"]["rust-version"]

    # The mise `rust` pin is a bare version string or a table with `version`.
    if isinstance(mise_rust := mise["tools"]["rust"], dict):
        mise_rust = mise_rust["version"]

    pairs = [
        (
            "README.md Rust badge ↔ Cargo.toml rust-version",
            badge("rust.svg"),
            major_minor(RUST)
        ),
        (
            "Cargo.toml rust-version ↔ .mise/config.toml rust pin",
            major_minor(RUST),
            major_minor(mise_rust)
        ),
        (
            "README.md Python badge ↔ crate/pyproject.toml requires-python",
            badge("python.svg"),
            major_minor(project["project"]["requires-python"])
        ),
        (
            ".mise/config.toml uv pin ↔ provision-uv action version",
            mise["tools"]["uv"],
            action_pin("provision-uv", "version")
        ),
        (
            ".mise/config.toml maturin pin ↔ build-wheel action maturin-version",
            mise["tools"]["maturin"],
            action_pin("build-wheel", "maturin-version")
        ),
        (
            ".mise/config.toml maturin pin ↔ crate/pyproject.toml build requirement",
            mise["tools"]["maturin"],
            build_requirement("maturin")
        ),
        (
            ".mise/config.toml wasm-bindgen pin ↔ Cargo.lock wasm-bindgen",
            mise["tools"]["github:rustwasm/wasm-bindgen"],
            cargo_lock_version("wasm-bindgen")
        )
    ]

    failed = 0
    for label, left, right in pairs:
        if left != right:
            print(f"::error::parity mismatch in {label}: {left!r} vs {right!r}")
            failed = 1

    raise SystemExit(failed)
