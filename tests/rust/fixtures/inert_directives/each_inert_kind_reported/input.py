def fetch(url: str) -> bytes:  # fmt: skip
    return download(url)


def parse(payload):  # prose: ignore
    return payload.decode()


def render(page: str) -> str:  # prose: ignore[signature-annotations]
    return page.strip()


def store(record: dict) -> None:  # prose: ignore[signature-annotation]
    archive(record)
