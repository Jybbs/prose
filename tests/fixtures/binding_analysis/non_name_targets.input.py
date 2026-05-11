"""
Assignment targets beyond plain `Expr::Name`. Attribute and
subscript targets walk the receiver and index as reads without
introducing a binding. Tuple, list, and starred targets recurse
into their contained names.
"""


class Holder:
    pass


obj      = Holder()
ix       = 0
xs       = [1, 2, 3]
obj.attr = 1
obj[ix]  = 2
a, b = (1, 2)
*head, tail = xs
[c, d] = (3, 4)
