class S:
    def _c(self, prospective, spec, prefix):
        return (self._compare_greater_than_equal(prospective, spec)) and (
            self._compare_equal(prospective, prefix)
        )
