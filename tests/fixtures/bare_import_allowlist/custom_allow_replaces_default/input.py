"""
A sidecar config replaces the default allowlist with `["torch"]`.
The default-allowlisted `numpy` is now flagged while `torch` is
preserved.
"""

import numpy
import torch
