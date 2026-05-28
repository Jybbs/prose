"""
A `TypedDict` class body has its annotated field declarations
alphabetized. Detection runs against the AST shape rather than the
base class name, so `class T(TypedDict)` and a hand-rolled annotated
class would both fire identically.
"""

from typing import TypedDict


class Config(TypedDict):
    timeout: int
    name: str
    retries: int
    host: str
