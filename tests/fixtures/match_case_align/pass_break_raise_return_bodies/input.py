"""
Arms exercising the full set of non-assignment collapsible
bodies (`Pass`, `Continue`, `Break`, `Raise`, `Expr`, `Return`),
one variant per arm. The rule aligns the colons across all six,
with `case _` padding out to the column the literal patterns
fix.
"""

def dispatch(token):
    match token:
        case "noop":
            pass
        case "skip":
            continue
        case "stop":
            break
        case "boom":
            raise RuntimeError("boom")
        case "echo":
            log(token)
        case _:
            return None
