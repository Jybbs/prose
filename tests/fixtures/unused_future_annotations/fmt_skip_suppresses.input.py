"""
A trailing `# fmt: skip` on the directive line drops the edit through
the pipeline's central suppression filter, leaving the directive in
place even though the file carries no annotations.
"""

from __future__ import annotations  # fmt: skip

x = 1
