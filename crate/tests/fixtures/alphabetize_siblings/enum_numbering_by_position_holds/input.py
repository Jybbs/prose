from enum import IntEnum, StrEnum, auto


class Boundary(StrEnum):
    STRICT = auto()
    CONFORM = auto()
    EJECT = auto()


class ParameterKind(IntEnum):
    POSITIONAL_ONLY = "positional-only"
    VAR_KEYWORD = "variadic keyword"
    KEYWORD_ONLY = "keyword-only"

    def __new__(cls, description):
        member = int.__new__(cls, len(cls.__members__))
        member._value_ = len(cls.__members__)
        return member
