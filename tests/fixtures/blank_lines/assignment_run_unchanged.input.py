"""
A run of module-level assignments leaves each adjacency at the
source's actual count, because the module-scope dispatch returns
`None` for assignment-after-assignment pairs.
"""

PORT = 8080
HOST = "localhost"
TIMEOUT: float = 30.0
