def outer():
    """Outer docstring."""

    def inner():
        """Inner docstring."""
        return 1

    return inner()
