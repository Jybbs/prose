"""
A `# fmt: off` block carries the same suppression weight against
lint diagnostics as it does against edits. The constant inside
the block escapes the rule's emission path.
"""

# fmt: off
PI = 3.14
# fmt: on
