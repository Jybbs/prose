"""
A dict whose values are themselves dicts. Each nested dict aligns
its own `:` column independently at its own indent, so the inner
dict's alignment is not affected by the outer dict's key widths.
"""

config = {
    "development": {
        "host": "localhost",
        "port_number": 8080,
    },
    "production": {
        "host": "prod.example.com",
        "port_number": 443,
    },
}
