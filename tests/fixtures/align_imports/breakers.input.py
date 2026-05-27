"""
Bare `import M` statements without aliases sit in the unified
block without contributing a member, since they carry no `as`
keyword to anchor on. Non-import statements between imports still
end the block, so the from-import below the assignment lands in
its own singleton block and drops.
"""

import collections as c
import datetime as dt
import os
import re as r
import json as j

from sys import path
LOG_LEVEL = "INFO"
from typing import Optional
