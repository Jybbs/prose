class Catalog:
    def update(self, target, source, *, retries=3, mode):
        """Apply ``source`` onto ``target``.

        Args:
            target: Mapping receiving the update.
            source: Mapping providing new values.
            retries: Attempts before giving up.
            mode: Merge strategy name.
        """
