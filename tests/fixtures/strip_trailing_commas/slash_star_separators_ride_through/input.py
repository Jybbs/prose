"""
The positional-only `/` and keyword-only `*` separators have no AST
node of their own, but the backward scan from the closing `)` only
asks whether the immediate previous non-trivia token is a comma, so
the separators ride through without any special handling.
"""


def split_pos_only(
    a,
    /,
) -> None:
    return None


def split_kw_only(
    *,
    b,
) -> None:
    return None
