"""
`del a, b, c` targets alphabetize because each delete is
independent of the others. The targets are scrubbed by source-
text key, mirroring how set literal elements sort.
"""

def cleanup():
    cache = {}
    buffer = []
    registry = {}
    del registry, cache, buffer
