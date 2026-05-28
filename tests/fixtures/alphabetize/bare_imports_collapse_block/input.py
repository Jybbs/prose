"""
Every bare `import` statement at a body's top level collapses into
a single alphabetized block. Blank lines that the user wrote between
imports are removed so the form reads as one paragraph regardless
of how the source was originally arranged.
"""

import zlib
import argparse
import os

import numpy as np
import pandas as pd
