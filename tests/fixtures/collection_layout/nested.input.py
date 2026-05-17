"""
Nested collections expand independently. A qualifying child serializes
through the parent's `expand` when the parent also qualifies. A
qualifying child inside a non-qualifying parent expands via `walk_expr`
falling through into the parent's descendants. Each child's
`code-line-length` check uses its own column, including the key-offset for
dict values, so tiered structures expand level-by-level only where
the inline form would overflow. Nesting precedence on collapse runs
outermost-first: when an outer multi-line literal fits inline its
inner literals move with it, and when the outer is pinned by its own
width the inner runs its collapse decision against its own column.
"""

cascade = [{"name": "alice_wonderland_the_explorer", "role": "administrator", "email": "alice.wonderland@example-company-domain.com"}, {"name": "bob", "role": "user"}]
tiered_dict = {"primary_database": {"host": "very-long-hostname.example.com", "port": 5432, "username": "admin", "password": "secret"}, "cache": {"ttl": 60}}
shallow_dict_in_dict = {"connection": {"host": "localhost", "port": 5432}, "cache": {"ttl": 60, "size": 1024}}
walks_through_singleton = [{"key_alpha": 1, "key_beta": 2, "key_gamma": 3, "key_delta": 4, "key_epsilon": 5, "key_zeta": 6}]
outer_collapses_with_inner = [
    1,
    [
        2,
        3
    ],
    4
]
outer_pinned_inner_collapses = [
    "first long string that pushes past the inline-form budget for the outer list",
    [
        2,
        3,
        5
    ],
    "another long string padding outer past the budget so the outer cannot collapse"
]
