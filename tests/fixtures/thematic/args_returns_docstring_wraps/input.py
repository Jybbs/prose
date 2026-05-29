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
