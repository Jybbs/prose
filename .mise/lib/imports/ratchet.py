"""
The ratchet on a width's breaks, meaning the frame set a baseline carries,
the modules whose break it already holds, and the set a run bakes for the
next.
"""

from json    import dumps, loads
from os      import environ
from pathlib import Path

from records import Width


def bake(widths: list[Width], path: str):
    """
    Write the file and reason of every frame each of `widths` breaks at to
    `path`, keyed by the width's label.
    """
    frames = {
        found.label: sorted({(brk.frame[0], brk.reason) for brk in found.breaks})
        for found in widths
    }
    Path(path).write_text(dumps(frames, indent=2) + "\n", encoding="utf-8")


def baseline() -> dict:
    """
    Return the break set `PROSE_IMPORTS_BASELINE` names, empty where unset.
    """
    named = environ.get("PROSE_IMPORTS_BASELINE")
    return loads(Path(named).read_text(encoding="utf-8")) if named else {}


def judge(found: Width, held: dict) -> set[str]:
    """
    Return the broken modules of `found` whose frame file and reason the
    baseline `held` carries for its width.
    """
    known = {tuple(key) for key in held.get(found.label, [])}
    return {brk.module for brk in found.breaks if (brk.frame[0], brk.reason) in known}
