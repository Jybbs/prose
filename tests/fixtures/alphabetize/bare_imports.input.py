"""
Bare `import` statement runs alphabetize within blank-line bounds,
mirroring the `from`-import-run pass on the `Stmt::Import` shape.
Bare imports sit above `from`-imports as their own group, and the
two run families never interleave.
"""

import zlib
import argparse
import os

import numpy as np
import pandas as pd
