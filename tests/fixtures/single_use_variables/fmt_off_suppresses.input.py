"""
A `# fmt: off` block drops the rule's diagnostic for any single-use
binding inside the suppressed span. The pipeline filters
`Severity::Lint` diagnostics by range alongside edits.
"""


# fmt: off
def basic(arg):
    x = expensive(arg)
    return x + 1
# fmt: on
