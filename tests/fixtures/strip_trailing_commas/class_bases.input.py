"""
A multi-line class base list drops its trailing comma. The class's
argument list shares the `Arguments` shape with function calls, and
the backward scan from before the closing `)` lands on the comma
after the last base.
"""


class Loader(
    BaseLoader,
    CacheMixin,
):
    pass
