"""
Comments attached above each method travel with the method when
its slot moves. Detached comments separated by a blank line stay
in source position because `Orderer`'s block-range detection stops
at the blank-line boundary.
"""

class Matcher:
    # describes the public match entry point
    def match(self, resume):
        pass

    # describes the constructor

    def __init__(self, clusters):
        pass

    # describes the private validator
    def _validate(self, resume):
        pass
