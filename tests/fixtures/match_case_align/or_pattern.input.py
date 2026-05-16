"""
Arms with `case A | B` pattern alternations. The third arm's
pattern is wide enough to break the greedy split-policy run,
falling into its own singleton sub-group. As the strictly widest
arm, it anchors the other two so all `:`s share one column.
"""

match status:
    case "ok" | "pass":
        result = True
    case "warn":
        result = True
    case "fail" | "error" | "panic":
        result = False
