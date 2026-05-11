"""
A class with its own docstring (here using triple-apostrophe quotes)
gets 1 blank line between the docstring and its first method. The
class-scope canonical fires on the (docstring, method) pair.
"""


class Posting:
    '''Inline class docstring using triple-apostrophe quotes.'''
    def m1(self):
        return 1



    def m2(self):
        return 2
