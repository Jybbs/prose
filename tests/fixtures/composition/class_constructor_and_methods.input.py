"""
A class whose `__init__` body opens with a run of self-attribute
assignments alignable on `=`, followed by methods declared out of
alphabetical order with missing blank-line spacing.

Rules:
- alphabetize
- blank_lines
- align_equals
"""


class Service:
    def __init__(self, host, port):
        self.host = host
        self.port_id = port
        self.is_ready = False
    def zeta(self):
        return self.host
    def alpha(self):
        return self.port_id
    def beta(self):
        return self.is_ready
