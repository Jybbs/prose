from __future__ import annotations

__lazy_modules__ = ["pkg.patterns"]

import sys

from pkg.patterns import NUMBER


def parse(text):
    return NUMBER.match(text) or sys.exit(1)


LIMIT = 10
