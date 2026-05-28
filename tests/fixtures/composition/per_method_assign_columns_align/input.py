"""
Class methods out of alphabetical order, each carrying a run of
assignments in its body. Method bodies' `=` columns align within
each method.

Rules:
- alphabetize
- blank_lines
- align_equals
"""


class Builder:
    def render(self):
        out = []
        markup_text = ""
        flag = True
        return out, markup_text, flag
    def configure(self):
        host = "localhost"
        port_id = 8080
        retries = 3
        return host, port_id, retries
