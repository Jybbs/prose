"""
A module-level run of single-name `Assign` statements with no
intra-run references alphabetizes within tier 0. Every binding's
RHS is a literal, so the dependency graph is empty and every name
sorts directly against every other.
"""

charlie = "third"
alpha = "first"
bravo = "second"
