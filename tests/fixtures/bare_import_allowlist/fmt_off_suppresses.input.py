"""
A `# fmt: off` block drops the rule's diagnostic for any bare import
inside the suppressed span. The pipeline filters `Severity::Lint`
diagnostics by range alongside edits.
"""

# fmt: off
import os
# fmt: on
