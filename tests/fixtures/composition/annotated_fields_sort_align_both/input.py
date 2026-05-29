class Endpoint:
    timeout: float = 30.0
    host: str = "localhost"
    retry_count: int = 3
    backend_label: str = "primary"
