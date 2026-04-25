"""
An `Args:` block with a malformed entry whose line starts with
punctuation rather than an identifier. The rule skips that line
when collecting alignment members, so the well-formed entries
above and below it still align with each other.
"""

def render(template: str, context: dict, partials: dict) -> None:
    """
    Renders a template.

    Args:
        template: source markup.
        : not a real entry, the parser leaves this line alone.
        context: runtime variables.
        partials: nested fragments.
    """
    _render(template, context, partials)
