"""
A trailing `# prose: ignore[loose-constants]` directive drops the
diagnostic the rule would otherwise emit. The post-loop lint
filter from per-line suppression runs against `Severity::Lint`
output by `Rule::lint`.
"""

PI = 3.14  # prose: ignore[loose-constants]
