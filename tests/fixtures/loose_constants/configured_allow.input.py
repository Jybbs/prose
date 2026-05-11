"""
A name listed in `[tool.prose.rules.loose-constants].allow` passes
through silently, allowing project conventions like `LOG_LEVEL` or
`DEFAULT_TIMEOUT` to publish at the module root.
"""

LOG_LEVEL       = "INFO"
DEFAULT_TIMEOUT = 30
OTHER           = 1
