"""
Three decorators bind values into the function signature
positionally and so block parameter alphabetization:
`pytest.mark.parametrize`, `hypothesis.given`, and
`click.argument`. Functions carrying any of these keep their
parameter order intact.
"""

import click
import hypothesis.strategies as st
import pytest


@pytest.mark.parametrize("a, b", [(1, 2), (3, 4)])
def test_add(b, a):
    pass


@hypothesis.given(st.integers(), st.text())
def test_thing(num, label):
    pass


@click.command()
@click.argument("source")
@click.argument("dest")
def copy(dest, source):
    pass
