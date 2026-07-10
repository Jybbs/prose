#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Audit the built wasm module against its declared size budget.

Reads the `[package.metadata.wasm] max-bytes` ceiling from
`wasm/Cargo.toml`, stats the `wasm-release` artifact, and exits 0 when
the module fits. An overage surfaces as an `::error::` annotation
naming both figures.
"""

from pathlib import Path
from tomllib import loads

ARTIFACT = Path("target/wasm32-unknown-unknown/wasm-release/prose_wasm.wasm")


if __name__ == "__main__":

    manifest  = loads(Path("wasm/Cargo.toml").read_text(encoding="utf-8"))
    max_bytes = manifest["package"]["metadata"]["wasm"]["max-bytes"]
    size      = ARTIFACT.stat().st_size

    if size > max_bytes:
        raise SystemExit(f"::error::wasm module {size} B exceeds {max_bytes} B budget")
