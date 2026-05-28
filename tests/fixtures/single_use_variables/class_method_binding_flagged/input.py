"""
A single-use binding inside a class method is flagged just like any
function-local binding. The rule walks into class bodies and processes
each method's scope independently.
"""


class Service:
    def consume(self, payload):
        normalized = transform(payload)
        return normalized
