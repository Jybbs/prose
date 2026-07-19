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
    Rust version    `README.md` badge vs `Cargo.toml` `rust-version`
    Rust toolchain  `Cargo.toml` `rust-version` vs `.mise/config.toml` pin
    Python version  `README.md` badge vs `crate/pyproject.toml` `requires-python`
"""

from pathlib import Path
from re      import search
from tomllib import loads


def action_uv_version() -> str:
    """
    Return the `version` input pinned in the `provision-uv` composite action.
    """
    text = Path(".github/actions/provision-uv/action.yml").read_text(encoding="utf-8")
    if match := search(r"version\s*:\s*(\S+)", text):
        return match.group(1)
    raise SystemExit("::error::no version pin in provision-uv/action.yml")


def badge(svg: str) -> str:
    """
    Return the `<major>.<minor>` token from the README badge line whose
    link target carries `svg`.
    """
    for line in Path("README.md").read_text(encoding="utf-8").splitlines():
        if svg in line and (match := search(r"(\d+\.\d+)\+", line)):
            return match.group(1)
    raise SystemExit(f"::error::no README.md badge line carries {svg!r}")


def cargo_lock_version(name: str) -> str:
    """
    Return the version `Cargo.lock` resolves for the `name` package.
    """
    for package in loads(Path("Cargo.lock").read_text(encoding="utf-8"))["package"]:
        if package["name"] == name:
            return package["version"]
    raise SystemExit(f"::error::no Cargo.lock package named {name!r}")


def major_minor(value: str) -> str:
    """
    Return `<major>.<minor>` from any string carrying a SemVer head.
    """
    if match := search(r"\d+\.\d+", value):
        return match.group(0)
    raise SystemExit(f"::error::cannot parse major.minor from {value!r}")


if __name__ == "__main__":

    cargo   = loads(Path("Cargo.toml").read_text(encoding="utf-8"))
    mise    = loads(Path(".mise/config.toml").read_text(encoding="utf-8"))
    project = loads(Path("crate/pyproject.toml").read_text(encoding="utf-8"))
    rust    = cargo["workspace"]["package"]["rust-version"]

    # The mise `rust` pin is a bare version string or a table with `version`.
    mise_rust = mise["tools"]["rust"]
    if isinstance(mise_rust, dict):
        mise_rust = mise_rust["version"]

    wasm_bindgen = cargo_lock_version("wasm-bindgen")
    pairs = [
        (
            "README.md Rust badge ↔ Cargo.toml rust-version",
            badge("rust.svg"),
            major_minor(rust)
        ),
        (
            "Cargo.toml rust-version ↔ .mise/config.toml rust pin",
            major_minor(rust),
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
            action_uv_version()
        ),
        (
            ".mise/config.toml wasm-bindgen pin ↔ Cargo.lock wasm-bindgen",
            mise["tools"]["github:rustwasm/wasm-bindgen"],
            wasm_bindgen
        )
    ]

    failed = 0
    for label, left, right in pairs:
        if left != right:
            print(f"::error::parity mismatch in {label}: {left!r} vs {right!r}")
            failed = 1

    raise SystemExit(failed)
