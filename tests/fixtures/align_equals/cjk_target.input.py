"""
CJK identifiers count as two display columns per character. Padding
math uses display columns, not bytes, so a two-character CJK target
sits four columns wide and the rule lines up its `=` with ASCII
neighbors by display position rather than by byte length.
"""

alpha = 1
名前 = 2
longer = 3
