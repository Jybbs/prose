"""
An annotated parameter without a default value splits the
surrounding parameter run. The remaining parameters have widths
that would align if grouped, but they stay unpadded as
singletons.
"""


def f(
    *,
    x: int = 1,
    flag: bool,
    verbose: bool = True,
):
    pass
