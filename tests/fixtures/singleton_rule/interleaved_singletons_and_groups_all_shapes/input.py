class Solo:
    only_field : str


class Pair:
    first  : str
    second : int


single_dict = {"key" : "value"}

paired_dict = {
    "first"  : 1,
    "second" : 2,
}


def lone_param(x : int) -> int:
    return x


def two_params(first : int, second : str) -> str:
    return ""
