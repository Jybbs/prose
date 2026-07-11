#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Audit the built wasm module against its declared size budget.

Reads the `[package.metadata.wasm] max-mib` ceiling from `wasm/Cargo.toml`,
stats the `wasm-release` artifact named by `WASM_ARTIFACT`, and exits 0
when the module fits. An overage surfaces as an `::error::` annotation
naming both figures in MiB.
"""

from os      import environ
from pathlib import Path
from tomllib import loads

MIB = 1024**2


if __name__ == "__main__":

    manifest = loads(Path("wasm/Cargo.toml").read_text(encoding="utf-8"))
    max_mib  = manifest["package"]["metadata"]["wasm"]["max-mib"]
    size     = Path(environ["WASM_ARTIFACT"]).stat().st_size

    if size > max_mib * MIB:
        raise SystemExit(
            f"::error::wasm module {size / MIB:.2f} MiB exceeds {max_mib} MiB budget"
        )
