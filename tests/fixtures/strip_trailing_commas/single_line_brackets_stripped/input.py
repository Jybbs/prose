"""
A container whose opening and closing brackets sit on the same
source line still drops its trailing comma. Single-line trailing
commas carry no semantic meaning across the in-scope contexts and
are stripped on the same backward-scan path that handles multi-line
input.
"""

call = f(a, b, c,)
literal = [1, 2, 3,]
mapping = {"a": 1, "b": 2,}


def signature(a, b, c,):
    return a + b + c
