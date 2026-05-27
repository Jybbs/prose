"""
A run already in tier-and-alphabetical order produces no edits,
so the second pass sees identical input and the rule is a no-op
end-to-end.
"""

ALPHA = 1
BRAVO = 2
CHARLIE = ALPHA + BRAVO
