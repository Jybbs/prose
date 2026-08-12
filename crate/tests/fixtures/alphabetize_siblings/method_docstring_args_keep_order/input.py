class Catalog:
    def update(self, target, source):
        """Apply ``source`` onto ``target``.

        Args:
            target: Mapping receiving the update.
            source: Mapping providing new values.

        Raises:
            ValueError: Mapping shapes disagree.
            KeyError: A required key is missing.
        """
