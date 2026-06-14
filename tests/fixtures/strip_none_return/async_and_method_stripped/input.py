class Service:
    def __init__(self) -> None:
        self.ready = False


async def shutdown() -> None:
    await cleanup()
