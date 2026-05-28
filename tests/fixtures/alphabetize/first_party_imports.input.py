"""
Lifts configured first-party imports into the local-package group.
`myapp` imports join relative imports as local-package, bare before
`from`, while unrelated packages stay in the bare and external
`from` groups.
"""

import os
from myapp.db import Session
import myapp.core
from collections import Counter
from . import shared
from myapp import app
