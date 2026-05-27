"""
A unified import block carrying bare and `from` imports sorts each
kind within its own contiguous slot. The two slots keep their
source-order positions until issue #137 lands the canonical order.
"""

import zlib
import argparse
from loguru import logger
from collections import Counter
