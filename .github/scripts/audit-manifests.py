#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Audit composite action manifests for constructs the runner refuses to load.

The workflow parser resolves YAML anchors whereas the action-manifest parser
rejects them outright, failing every job that loads the action before any step
runs. Anchors are therefore legal under `.github/workflows` and never inside an
action manifest. Findings surface as `::error::` annotations naming the file
and the anchor.
"""

from pathlib import Path
from re      import MULTILINE, finditer


if __name__ == "__main__":

    failed = 0
    for manifest in sorted(Path(".github/actions").glob("*/action.yml")):
        text = manifest.read_text(encoding="utf-8")
        for match in finditer(r"^(\s*[\w-]+)\s*:\s*&([\w-]+)", text, MULTILINE):
            line = text[: match.start()].count("\n") + 1
            print(
                f"::error file={manifest},line={line}::"
                f"the action-manifest parser rejects YAML anchors, so remove {match.group(2)!r}"
            )
            failed = 1

    raise SystemExit(failed)
