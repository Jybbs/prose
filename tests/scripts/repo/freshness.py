"""
Pins that tasks decide freshness from the contents of their `sources`, so a
branch switch rewriting unchanged files keeps them current, and that a
`restore-wasm` hit returns the job to modification times, since a runner
holds no recorded hashes and hashing would run `wasm:pack` over restored
outputs.
"""

from pathlib import Path
from tomllib import loads
from yaml    import safe_load

ROOT   = Path(__file__).resolve().parents[3]
ACTION = ROOT / ".github/actions/restore-wasm/action.yml"
HASHED = "MISE_TASK_SOURCE_FRESHNESS_HASH_CONTENTS"


def test_config_hashes_source_contents():
    """
    Pins that the mise config turns on content hashing for every task.
    """
    config = loads((ROOT / ".mise/config.toml").read_text(encoding="utf-8"))
    assert config["settings"]["task"]["source_freshness_hash_contents"] is True


def test_restore_hit_returns_to_modification_times():
    """
    Pins that the `restore-wasm` step running on a cache hit both stamps the
    restored files and turns content hashing off for the later steps of the
    job, so `wasm:pack` keeps skipping the `prose_wasm` compile.
    """
    steps = safe_load(ACTION.read_text(encoding="utf-8"))["runs"]["steps"]
    stamp = next(step for step in steps if "touch" in step.get("run", ""))
    lines = stamp["run"].splitlines()

    assert stamp["if"] == "${{ steps.entry.outputs.cache-hit == 'true' }}"
    assert f'echo {HASHED}=false >> "$GITHUB_ENV"' in lines
    assert "find site/public/wasm -type f -exec touch {} +" in lines
