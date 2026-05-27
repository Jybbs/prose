"""
Module-level constants reorder around a `# fmt: off` block. The
assigns above the block sort alphabetically and align their `=`
columns, the block stays verbatim because the suppression
directive bounds its own scope, and the run boundary respects
the bracket.

Rules:
- align-equals
- alphabetize
"""

zebra = 1
foo = 2
bar_baz = 3

# fmt: off
matrix = [[0.7, 0.1, 0.1],
          [0.1, 0.7, 0.1],
          [0.1, 0.1, 0.7]]
# fmt: on
