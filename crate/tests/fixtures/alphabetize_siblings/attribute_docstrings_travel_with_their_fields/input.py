from pydantic import BaseModel


class Settings(BaseModel):
    zeta: int = 1
    """Zeta count."""
    mid: float = 0.5
    alpha: str = "a"
    """Alpha name."""
