"""
Bare imports outside the default allowlist arrive out of order.
alphabetize reorders the statements and bare-import-allowlist
emits a diagnostic per offending import whose range tracks the
post-reorder offset.

Rules:
- alphabetize
- bare-import-allowlist
"""

import sys
import os
import json
