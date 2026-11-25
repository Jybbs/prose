from dataclasses import dataclass


@dataclass
class Posting:
    title: str
    company: str

    def summary(self):
        return self.title

    def render(self):
        return self.company
