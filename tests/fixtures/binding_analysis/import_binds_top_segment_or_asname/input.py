"""
`import` binds the top-level module segment by default and the
explicit asname when present. `from ... import` binds each name
(or its asname) without the dotted-path step.
"""


import a
import b.c.d
import e.f as g
from h import i
from j import k as l
from __future__ import annotations
