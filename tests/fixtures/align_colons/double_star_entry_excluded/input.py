defaults = {"host": "localhost", "port": 8080, "timeout": 30}

overrides = {
    "host": "example.com",
    **defaults,
    "user_agent": "prose/0.1",
    "proxy": None,
}
