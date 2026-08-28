def check_builtin(value):
    return value


def check_choice(value):
    return value


class Option:
    TYPE_CHECKER = {"choice": check_choice, "int": check_builtin}
