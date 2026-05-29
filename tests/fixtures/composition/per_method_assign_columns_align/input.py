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
