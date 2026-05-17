"""
A class method with `self` and four typed parameters trips the count
trigger and expands at the class-body indent. Pins the walker
descending into class bodies the same way it descends into top-level
defs, with the indent derived from the method's own `def` position.
"""


class Renderer:
    def render(self, layout: tuple[int, int], palette: str, spread: float, target: int):
        return (self, layout, palette, spread, target)
