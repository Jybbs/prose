class Reader(Protocol):
    def close(self) -> None:
        """Close the reader."""
        ...
