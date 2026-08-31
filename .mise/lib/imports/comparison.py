"""
Comparing what two trees left behind, meaning why one module's run counts as
broken beside the original, and which modules of a sweep broke at all.
"""

from records import Break, Outcome

MISSING = "no plain constant"


def compare(
    after   : dict[str, Outcome],
    before  : dict[str, Outcome],
    modules : list[str]
) -> tuple[list[Break], list[str], list[str]]:
    """
    Return the breaks among `modules` between what the original tree left
    in `before` and the formatted tree in `after`, the modules comparable at
    all, and the ones a run left unmeasured.
    """
    comparable = [
        module
        for module in modules
        if before[module].kind == "ok" and after[module].kind != "unmeasured"
    ]

    return (
        [
            Break(
                formatted = after[module],
                module    = module,
                name      = differs[1],
                original  = before[module],
                reason    = differs[0]
            )
            for module in comparable
            if (differs := divergence(after[module], before[module]))
        ],
        comparable,
        [
            module
            for module in modules
            if "unmeasured" in (before[module].kind, after[module].kind)
        ]
    )


def divergence(formatted: Outcome, original: Outcome) -> tuple[str, str | None] | None:
    """
    Return why `formatted` counts as broken beside `original` and the name
    it hinges on, or `None` where both bind the same namespace.
    """
    if formatted.kind != "ok":
        return formatted.error, formatted.name

    bound, rebound = set(original.names), set(formatted.names)
    if unbound := bound - rebound:
        name = min(unbound)
        return f"leaves `{name}` unbound", name

    if extra := rebound - bound:
        name = min(extra)
        return f"binds `{name}` the original does not", name

    before, after = dict(original.constants), dict(formatted.constants)
    if differing := {name for name, _ in before.items() ^ after.items()}:
        name     = min(differing)
        was, now = before.get(name, MISSING), after.get(name, MISSING)
        return f"binds `{name}` to {now} where the original binds {was}", name

    return None
