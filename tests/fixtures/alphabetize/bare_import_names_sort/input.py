"""
Multi-name `import a, b, c` aliases alphabetize within the
statement, mirroring the from-import alias sort. Single-name
imports are the preferred form and `align_imports` skips multi-name
imports, so this case is rare but covered for completeness.
"""

import zlib, argparse, os
