"""Export the grammar."""

# Python imports
import os

# Local imports
from .pgen2 import token
from .pgen2 import driver
from . import pytree

GRAMMAR = driver.load(os.path.join(pytree.root, "Grammar.txt"))
