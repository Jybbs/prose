"""
A single-use binding captured by a nested closure is flagged. The
binding analyzer attributes the closure's read to the outer scope,
so the binding looks single-use to the rule. Inlining changes
evaluation timing when the right-hand side carries side effects, so
the user is expected to review the diagnostic before acting.
"""


def factory():
    helper = compute()
    return lambda x: x * helper
