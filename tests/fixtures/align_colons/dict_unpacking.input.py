"""
A dict mixing `key: value` entries with `**other_dict` unpackings.
The unpacking entries do not participate in the colon-alignment
group because they have no key expression, so only the remaining
keyed entries align against each other.
"""

defaults = {"host": "localhost", "port": 8080, "timeout": 30}

overrides = {
    "host": "example.com",
    **defaults,
    "user_agent": "prose/0.1",
    "proxy": None,
}
