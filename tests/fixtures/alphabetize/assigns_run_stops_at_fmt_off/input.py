"""
A module-level run terminates at a `# fmt: off` block. The
assigns above the block sort within their run, the suppression
directive bounds its own scope so the block stays verbatim, and
no run forms across the bracket.
"""

zebra = 1
foo = 2
bar_baz = 3

# fmt: off
matrix = [[0.7, 0.1, 0.1],
          [0.1, 0.7, 0.1],
          [0.1, 0.1, 0.7]]
# fmt: on
