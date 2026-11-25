class Service:
    def consume(self, payload):
        normalized = transform(payload)
        return normalized
