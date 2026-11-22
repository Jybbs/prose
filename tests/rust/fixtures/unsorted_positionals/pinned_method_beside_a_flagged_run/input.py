from dataclasses import dataclass


@dataclass
class Report:
    zebra: int
    apple: int

    @click.argument("path")
    def render(self, target, source):
        return target
