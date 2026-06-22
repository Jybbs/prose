mixed = {
    "version"  : "0.1.0",
    "tags"     : ["staging", "ingest"],
    "name"     : "prose",
    "config": {
        "verbose"    : False,
        "strict"     : True,
        "tracebacks" : "short",
    },
    "author"   : "jibbs",
    "metadata" : load_metadata(),
    "deps"     : ["ruff_python_parser", "clap"],
}

inline = {"version": "0.1.0", "config": {"strict": True}, "name": "prose"}

with_spread = {
    "version" : "0.1.0",
    **defaults,
    "name"    : "prose",
    "config": {
        "strict"  : True,
        "verbose" : False,
        "tracing" : "off",
    },
}

single_line_only = {
    "config"   : {"strict": True},
    "metadata" : load_metadata(),
    "version"  : "0.1.0",
    "deps"     : ["ruff_python_parser", "clap"],
}

deep_nesting = {
    "delta" : "...",
    "alpha" : 1,
    "settings": {
        "network": {
            "timeout" : 30,
            "retries" : 5,
        },
        "logging": {
            "format": {
                "json"    : True,
                "compact" : False,
            },
            "stderr" : "warn",
            "stdout" : "info",
        },
    },
    "beta"  : 2,
}
