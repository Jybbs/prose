async def outer(layout: tuple[int, int], palette: str, spread: float, target: int, verbose: bool):
    def inner(host: str, port: int, retries: int, timeout: float, verbose: bool):
        return (host, port, retries, timeout, verbose)
    return await inner(palette, 8080, 3, 10.0, verbose)
