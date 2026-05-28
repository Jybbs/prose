"""
A for-loop target binds in the enclosing scope and may be a single
name or a tuple destructuring. The iter is visited as a read.
"""


xs = [(1, 2), (3, 4)]
for i in xs:
    print(i)
for a, b in xs:
    print(a + b)
