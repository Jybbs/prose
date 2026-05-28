"""
A trailing `# prose: keep` comment on the opening `{` line preserves
source order for that single dict. The unmarked sibling alphabetizes
and partitions through the same machinery, so the contrast lands side
by side: same scrambled input, two different outputs.
"""

unmarked = {
    "version"  : "0.1.0",
    "config"   : {
        "strict"  : True,
        "verbose" : False,
    },
    "name"     : "prose",
}

marked = {  # prose: keep
    "version"  : "0.1.0",
    "config"   : {
        "strict"  : True,
        "verbose" : False,
    },
    "name"     : "prose",
}
