def build(spec):
    return spec


# Filters *********************************************************************#

class FilterError(Exception):
    code = 1


FILTER_ERRORS = (FilterError, OSError)


def apply_filter(member):
    return member
