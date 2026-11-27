"""
Pins that every tool a task calls comes from a `[tools]` pin, and that the
lockfile holds an entry for each pin on every platform its `os` allows.
"""

from pathlib import Path
from pytest  import mark
from tomllib import loads

ROOT  = Path(__file__).resolve().parents[3]
LOCK  = loads((ROOT / ".mise/mise.lock").read_text(encoding="utf-8"))["tools"]
TOOLS = loads((ROOT / ".mise/config.toml").read_text(encoding="utf-8"))["tools"]

WANTED = {
    name.removeprefix("platforms.")
    for entries in LOCK.values()
    for name in entries[0]
    if name.startswith("platforms.")
}


def allowed(os: list[str] | None, platform: str) -> bool:
    """
    Returns whether the `os` list of a pin admits `platform`, which joins an
    operating system, an architecture, and any variant with hyphens.
    """
    system, arch, *_ = platform.split("-")
    return os is None or any(item in (system, f"{system}/{arch}") for item in os)


def holds(tool: str) -> bool:
    """
    Returns whether the lockfile records a platform for `tool`, which a tool
    built from source (`cargo:`, `rust`) does not.
    """
    return any(name.startswith("platforms.") for name in LOCK[tool][0])


@mark.parametrize("tool", sorted(name for name in TOOLS if holds(name)))
def test_lockfile_holds_each_pin_on_every_platform_its_os_allows(tool: str):
    """
    Pins that a tool's lock entry records each platform its `os` restriction
    admits and none it excludes, so a locked install on a recorded platform
    finds an entry or skips the tool.
    """
    pin  = TOOLS[tool]
    os   = pin.get("os") if isinstance(pin, dict) else None
    held = {
        name.removeprefix("platforms.")
        for name in LOCK[tool][0]
        if name.startswith("platforms.")
    }

    assert {p for p in WANTED if allowed(os, p)} <= held
    assert not {p for p in held if not allowed(os, p)}


def test_gh_comes_from_a_pin():
    """
    Pins that `gh`, which `gha:cut`, `gha:prune`, `lock:lift`,
    `repo:labels`, and `repo:rulesets` call, is pinned in `[tools]` and
    locked.
    """
    assert "aqua:cli/cli" in TOOLS
    assert "aqua:cli/cli" in LOCK
