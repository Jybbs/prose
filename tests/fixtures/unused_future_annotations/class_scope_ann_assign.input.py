"""
A class-body `x: int` annotation references the `int` builtin, which
the module-scope `BindingAnalysis` does not record. Trigger 3 fails
conservatively, leaving the directive in place even though Python
would resolve `int` at runtime.
"""

from __future__ import annotations


class Config:
    x: int
    y: int
