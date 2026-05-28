"""
A bare `import os as o` is flagged on the top-level segment `os`.
The alias `o` does not affect the lookup because the allowlist
matches the imported module name, not its local binding.
"""

import os as o
