class Base:
    def visit(self):
        return 1


class Outer:
    class Inner(Base):
        def visit(self):
            return super(Inner, self).visit()
