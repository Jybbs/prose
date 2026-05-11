"""
A dotted bare `import numpy.linalg` is preserved because the top-level
segment `numpy` sits in the allowlist. Submodule access inherits the
parent's allowlist membership.
"""

import numpy.linalg
