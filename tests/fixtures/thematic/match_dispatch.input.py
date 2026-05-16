"""
Cross-rule composition over a match statement: match_case_align
splits the three arms whose collapsed form would exceed 88 columns
and collapses the under-budget fallback arm, alphabetize sorts the
kwargs inside the dispatched calls, strip_trailing_commas removes
the dangling commas the source carries, and the singleton rule
strips the lone-key dict's pre-`:` padding. The full pipeline
running twice produces no further change.
"""

def dispatch(event):
    match event.kind:
        case "create": return Counter(timestamp=event.ts, source=event.src, action="create")
        case "update": return Counter(timestamp=event.ts, source=event.src, action="update")
        case "delete": return Counter(timestamp=event.ts, source=event.src, action="delete")
        case _      : return None


SETTINGS = {
    "default_action" : "noop",
}


def required_only(name):
    return {name,}
