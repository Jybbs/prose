"""
A `# prose: ignore[bare-import-allowlist]` trailing comment suppresses
the diagnostic for that line. The pipeline's lint-suppression pass
drops the entry by `(line, rule)`.
"""

import os  # prose: ignore[bare-import-allowlist]
