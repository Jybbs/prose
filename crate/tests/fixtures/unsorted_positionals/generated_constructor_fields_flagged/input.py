from dataclasses import dataclass


@dataclass
class Window:
    width: int
    height: int
    title: str = "untitled"
