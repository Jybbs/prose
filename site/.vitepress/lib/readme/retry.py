from pydantic import BaseModel


class RetryPolicy(BaseModel):

    attempts: int = 3
    backoff: float = 0.5
    max_delay: float | None = None
    retry_on: tuple[int, ...] = (429, 502, 503)


STRICT = RetryPolicy(retry_on=(429, 503), max_delay=30.0, backoff=1.5, attempts=5)
LOGGING = {"level": "INFO", "format": "json", "color": False, "backtrace": True}


def delay_for(attempt: int, policy: RetryPolicy) -> float:
    delay = policy.backoff * 2**attempt
    return min(delay, policy.max_delay or delay)
