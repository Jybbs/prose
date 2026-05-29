import pytest


@pytest.mark.parametrize("a, b", [(1, 2), (3, 4)])
def test_add(b, a, *, verbose=False, atomic=True, retries=3):
    pass
