from dataclasses import dataclass


class Base:
    def label(self):
        return ""


@dataclass(slots=True)
class Tag(Base):
    name: str = ""

    def label(self):
        return super(Tag, self).label()
