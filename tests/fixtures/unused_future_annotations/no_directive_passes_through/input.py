"""
A file carrying no `from __future__ import annotations` directive
passes through unchanged on every pass.
"""

import os


def main():
    print(os.getcwd())
