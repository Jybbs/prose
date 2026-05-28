"""
Top-level function with a long-form docstring carrying free prose,
an `Args:` section, and a `Returns:` section, plus a typed
parameter list of varying widths. The full pipeline reshapes the
docstring opener and closer onto their own lines, wraps the long
prose paragraph at the docstring budget, settles the blank-line
cushion around the function definition, and aligns the parameter
`:` columns within the signature.
"""

def render(template, context, encoding):
    """Render a template against the provided context, applying the configured fallback when context is missing a key.

    Args:
        template: The compiled template.
        context: Mapping of names to values.
        encoding: Output encoding.

    Returns:
        The rendered string in the requested encoding.
    """
    return template
