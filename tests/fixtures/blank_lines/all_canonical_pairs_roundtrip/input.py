def alpha():
    return 1


class Posting:
    """
    Class docstring binds to the class header with no blank line.
    """

    def first(self):
        return 1

    def second(self):
        return 2


class Sized:

    capacity: int = 0

    def grow(self):
        self.capacity += 1


def beta(value):

    match value:
        case _:
            return value
