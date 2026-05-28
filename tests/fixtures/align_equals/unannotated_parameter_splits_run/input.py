"""
An unannotated parameter splits the parameter run. The
surrounding annotated parameters have widths that would align if
grouped, but they stay unpadded as singletons.
"""


def f(
    x: int = 1,
    unannotated=2,
    verbose: bool = True,
):
    pass
