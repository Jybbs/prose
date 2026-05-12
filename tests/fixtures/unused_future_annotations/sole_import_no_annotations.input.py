"""
The file carries no annotations on any function or assignment, so the
`from __future__ import annotations` directive is unused regardless of
target Python version. The sole import is removed along with the blank
line that becomes superfluous.
"""

from __future__ import annotations

x = 1


def main():
    print(x)
