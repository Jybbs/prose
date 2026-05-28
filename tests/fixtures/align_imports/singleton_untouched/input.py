"""
A single import (or any singleton group) is left untouched, since
alignment requires at least two members per group. The lone
`from`-import and the lone `import`-as below are each their own
singleton.
"""

from collections import OrderedDict

import json as j
