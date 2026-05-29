class Service:
    def primary(self):
        """The primary entry point.
        Returns the configured default."""
        return self.default
    def secondary(self):
        """The secondary entry point.
        Returns the alternate fallback."""
        return self.fallback
