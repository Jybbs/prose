"""
A trailing `# prose: ignore[single-use-variables]` directive drops the
diagnostic the rule would otherwise emit. The post-loop lint filter
from per-line suppression runs against `Severity::Lint` output by
`Rule::lint`.
"""


def basic(arg):
    x = expensive(arg)  # prose: ignore[single-use-variables]
    return x + 1
