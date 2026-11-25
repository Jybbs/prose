from dataclasses import dataclass


@dataclass
class Report:
    width: int
    height: int

    def render(self, target, source):
        return target
