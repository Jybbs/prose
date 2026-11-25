from dataclasses import dataclass


@dataclass
class Outer:
    zeta: int
    alpha: int

    class Inner:
        yankee: int
        bravo: int
