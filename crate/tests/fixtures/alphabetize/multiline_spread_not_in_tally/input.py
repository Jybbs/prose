config = {
    "limits": {
        "timeout": 30,
        "retries": 5,
    },
    **load_defaults(
        primary_source,
        fallback_source,
    ),
    "version": "0.1.0",
}
