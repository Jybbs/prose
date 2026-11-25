def describe(duplicates):
    if duplicates:
        alias_details = ", ".join(["%s -> %s" % (alias, name) for (alias, name) in duplicates])
