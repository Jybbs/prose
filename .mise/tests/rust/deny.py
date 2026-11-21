"""
Pins the three `cargo deny` runs `rust:deny` makes and the policy and
targets each one reads.
"""

from collections.abc import Callable
from pathlib         import Path
from subprocess      import run
from tomllib         import loads

ROOT = Path(__file__).resolve().parents[3]


def test_deny_runs_each_check_under_its_policy_and_targets(
    stand_in: Callable[..., Path]
):
    """
    Pins that `rust:deny` runs advisories, bans, and sources under
    `.cargo/deny.toml`, then advisories under a copy reporting every
    unmaintained crate as a warning, then licenses over each target
    `.github/scripts/platforms.toml` names and `wasm32-unknown-unknown`.
    """
    stand_in(
        "cargo",
        "shift\n"
        "while [ $# -gt 0 ]; do\n"
        '  if [ "$1" = --config ]; then grep "^unmaintained" "$2"; shift; '
        "else printf '%s\\n' \"$1\"; fi\n"
        "  shift\n"
        "done\n"
    )
    platforms = loads(
        (ROOT / ".github/scripts/platforms.toml").read_text(encoding="utf-8")
    )["platforms"]
    targets = [
        f"--target={platform['target']}"
        for platform in platforms
        if "target" in platform
    ]
    finished = run(
        [ROOT / ".mise/tasks/rust/deny"],
        capture_output = True,
        check          = True,
        cwd            = ROOT,
        text           = True
    )

    assert finished.stdout.splitlines() == [
        "--locked", "check", "advisories", "bans", "sources", 'unmaintained = "all"',
        "--locked", "check", "--warn", "unmaintained", "advisories", "--locked",
        *targets,
        "--target=wasm32-unknown-unknown", "check", "licenses"
    ]
