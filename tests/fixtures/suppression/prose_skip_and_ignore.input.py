"""
A line can carry both `# prose: skip[<rule>]` and
`# prose: ignore[<rule>]` directives. The skip suppresses the
named format rule's edit, the ignore suppresses the named lint
rule's diagnostic, and every other rule continues to fire.
"""

DEFAULT_LIMIT = 50
RETRY_INTERVAL = func(3, 4,)  # prose: skip[strip-trailing-commas]  # prose: ignore[loose-constants]
PAGE_SIZE = 25
