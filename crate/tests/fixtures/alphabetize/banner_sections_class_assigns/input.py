class Config:
    # --- Network ---
    TIMEOUT = 30
    CACHE_TTL = TIMEOUT * 2
    PORT = 8080
    # --- Storage ---
    root: str = "/"
    backend: str = "s3"
