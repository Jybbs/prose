"""
A run containing a destructuring target skips entirely. Tuple
unpacking binds multiple names from one statement, and reorder
would risk a sortable item referencing those names landing before
the unpacking.
"""

zebra = 1
(alpha, beta) = (2, 3)
delta = 4
