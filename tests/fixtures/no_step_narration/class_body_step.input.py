"""
A numbered-step comment inside a class body is flagged the same way as
inside a function body. The own-line filter passes, the scope does not
affect the match.
"""


class Processor:
    # 1. normalize input
    def run(self, payload):
        return payload.strip()
