#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Import each module of a corpus before and after formatting and report the
modules the rewrite breaks, each attributed to the frame it raises in and
the rules whose fixes reach it.

Usage: imports/__main__.py <binary> <python> <corpus | module | "">

`PROSE_IMPORTS_WIDTHS` adds widths beside the default,
`PROSE_IMPORTS_TIMEOUT` bounds one module's run in seconds,
`PROSE_IMPORTS_BAKE` names a file the break set is written to, and
`PROSE_IMPORTS_BASELINE` names one an earlier run wrote, so only a break it
does not carry fails the run.
"""

from os  import environ
from sys import argv

from ratchet import bake, baseline, judge
from report  import render
from sweep   import Sweep

if __name__ == "__main__":

    binary, python, target = argv[1:]
    sweep  = Sweep(binary, python, target)
    widths = [None, *map(int, environ.get("PROSE_IMPORTS_WIDTHS", "").split())]
    held   = baseline()

    print(
        f"corpus      {sweep.corpus}\n"
        f"binary      {binary}\n"
        f"interpreter {python} ({sweep.version})\n"
        f"stage       {sweep.stage.root}",
        flush = True
    )

    results = []
    fresh   = False
    for width in widths:
        found   = sweep.sweep(width)
        carried = judge(found, held)
        report  = render(found, carried, sweep.corpus)
        fresh  |= any(brk.module not in carried for brk in found.breaks)
        print(f"\nwidth {found.label}\n{report}", flush=True)
        results.append(found)

    print(f"\neach tree survives under {sweep.stage.root}")
    if any(found.unmeasured for found in results):
        raise SystemExit(
            "a run left modules unmeasured, so the uncomparable count cannot be named"
        )
    if baked := environ.get("PROSE_IMPORTS_BAKE"):
        bake(results, baked)
        print(f"break set baked into {baked}")
        raise SystemExit(0)
    raise SystemExit(int(fresh))
