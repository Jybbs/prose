"""
A class carrying both annotated fields and methods, all declared
out of order, with missing blank-line spacing between methods.

Rules:
- alphabetize
- blank_lines
- align_colons
- align_equals
"""


class Service:
    timeout: float = 30.0
    host: str = "localhost"
    retry_count: int = 3
    def shutdown(self):
        return self._cleanup()
    def restart(self):
        return self.shutdown()
    def _cleanup(self):
        return None
