"""
The ratchet on a width's breaks, meaning the frame set a baseline carries,
the modules whose break it already holds, the set a run bakes for the next,
and the exit status the run ends on.
"""

from json    import dumps, loads
from os      import environ
from pathlib import Path

from records import Width


def bake(path: str, widths: list[Width]):
    """
    Write the file and reason of every frame each of `widths` breaks at to
    `path`, keyed by the width's label.
    """
    Path(path).write_text(
        dumps(
            {
                found.label: sorted(
                    {brk.key for brk in found.breaks}
                )
                for found in widths
            },
            indent = 2
        ) + "\n",
        encoding = "utf-8"
    )


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

    return {brk.module for brk in found.breaks if brk.key in known}


def verdict(results: list[tuple[Width, set[str]]]) -> int | str:
    """
    Return the run's exit status, the message where a run left a module
    unmeasured, zero after baking where `PROSE_IMPORTS_BAKE` names a file,
    and otherwise whether any width breaks a module its baseline does not
    carry.
    """
    if any(found.unmeasured for found, _ in results):
        return (
            "a run left modules unmeasured, so the uncomparable count cannot be named"
        )

    if baked := environ.get("PROSE_IMPORTS_BAKE"):
        bake(baked, [found for found, _ in results])
        print(f"break set baked into {baked}")
        return 0

    return int(
        any(
            brk.module not in carried
            for found, carried in results
            for brk in found.breaks
        )
    )
