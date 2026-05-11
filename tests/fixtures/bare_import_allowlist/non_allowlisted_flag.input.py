"""
A bare `import os` is flagged because `os` is not in the default
allowlist of `numpy` and `pandas`. The rule emits a `Severity::Lint`
diagnostic and leaves the source text untouched.
"""

import os
