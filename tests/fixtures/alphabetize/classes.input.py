"""
Top-level classes alphabetize among consecutive class siblings. A
non-class statement between two class blocks breaks the run, so each
contiguous class run sorts independently of statements that pin in
place around it.
"""

class Gamma:
    pass


class Alpha:
    pass


class Beta:
    pass
