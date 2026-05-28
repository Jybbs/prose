"""
Set literal elements alphabetize because Python sets are
mathematically unordered. The sort key is each element's source
text, so mixed-type sets sort by source representation. Star
unpacks (`*defaults`) pin in their source slot to preserve the
visual blend with the rest of the set.
"""

allowed = {"text", "html", "markdown"}

priorities = {3, 1, 2}

mixed = {
    "z",
    *defaults,
    "a",
    "m",
}
