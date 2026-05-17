"""
Each of the six aligned comparison operators forms its own group
when mixed inside one chain. Six independent singletons emit no
edits.
"""

if (
    a == 1
    and b != 2
    and c < 3
    and d <= 4
    and e > 5
    and f >= 6
):
    pass
