class Base:
    def apply(self):
        return 1


class Child(Base):
    def apply(self):
        return super(
            Child,  # the defining class
            self
        ).apply()
