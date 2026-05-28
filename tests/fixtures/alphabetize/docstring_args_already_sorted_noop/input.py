"""
An `Args:` section whose entries already sit in alphabetical order
passes through untouched.
"""


def render(context, encoding, template):
    """Render the template.

    Args:
        context: Mapping of names to values.
        encoding: Output encoding.
        template: The compiled template.
    """
    return template
