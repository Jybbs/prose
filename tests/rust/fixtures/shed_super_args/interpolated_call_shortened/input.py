class Base:
    def label(self):
        return ""


class Child(Base):
    def label(self):
        return f"{super(Child, self).label()}!"
