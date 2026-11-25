class WithMethod:
    def alpha(self):
        return self


class WithDecoratedMethod:
    @classmethod
    def builder(cls):
        return cls()


class WithAnnotatedField:
    capacity: int = 0


class WithBareField:
    label = "unset"


class WithNestedClass:
    class Inner:
        def m(self):
            return self
