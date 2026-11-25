report_invalid_enum_member_names("invalid enum member name(s) %s, aborting" % (
        ", ".join(repr(name) for name in invalid_names if name not in allowed)
        ))
