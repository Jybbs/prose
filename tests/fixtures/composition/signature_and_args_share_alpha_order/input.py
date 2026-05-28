"""
Function with parameters and matching Args entries both in
non-alphabetical source order, with long descriptions.
`alphabetize` reorders the signature and the Args entries against
the same alphabetical key, `align_colons` aligns every colon in
both the signature and the Args section, and `docstring_wrap`
wraps each Args description at the post-align hanging column.
Signature and Args entries land in matching alphabetical order.

Rules:
- alphabetize
- align_colons
- docstring_wrap
"""


def render(template, context_map, escape_html):
    """
    Summary line above the structured section.

    Args:
        template: A very long description of the template parameter that should wrap at the post-align hanging column with hanging indent under the description.
        context_map: A mapping of identifier names to values for the rendering context, long enough to also need wrapping at the hanging column.
        escape_html: Whether to escape HTML output for safety, a description long enough to wrap at the same hanging column as the others.
    """
    return template
