"""
A genuine singleton block (just one import statement in the file)
fails the `members.len() >= 2` gate and so emits no edit. The lone
gap stays untouched.
"""

from collections import OrderedDict
