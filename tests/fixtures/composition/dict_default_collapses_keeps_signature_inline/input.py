"""
A function default argument is a multi-line dict literal whose
collapsed inline form keeps the whole signature under
`Config::code_line_length`. `collection_layout` collapses the dict,
`align_colons` no longer applies because the dict is now single-line,
and `signature_layout` sees the shortened signature line and leaves
it inline rather than expanding to one-parameter-per-line. The full
pipeline running twice produces no further change.

Rules:
- collection-layout
- align-colons
- signature-layout
"""


def configure(timeout=30, options={
    "alpha" : 1,
    "beta"  : 2
}, verbose=True):
    return (timeout, options, verbose)
