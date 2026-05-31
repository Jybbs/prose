class Service:
    def fetch(self, timeout : float) -> bytes:
        return b""

    def store(self, *, ttl : int = 3600) -> None:
        pass

    def render(self, template : str, context : dict) -> str:
        return ""
