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
