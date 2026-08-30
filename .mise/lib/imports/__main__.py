#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.14"
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

from ratchet import baseline, judge, verdict
from report  import banner, render
from sweep   import Sweep

binary, python, target = argv[1:]
sweep  = Sweep(binary, python, target)
widths = [None, *map(int, environ.get("PROSE_IMPORTS_WIDTHS", "").split())]
held   = baseline()

print(banner(sweep), flush=True)
results = []  # prose: ignore[miscased-constants]
for width in widths:
    found   = sweep.sweep(width)
    carried = judge(found, held)
    report  = render(carried, sweep.corpus, found)
    print(f"\nwidth {found.label}\n{report}", flush=True)
    results.append((found, carried))

print(f"\neach tree survives under {sweep.stage.root}")
raise SystemExit(verdict(results))
