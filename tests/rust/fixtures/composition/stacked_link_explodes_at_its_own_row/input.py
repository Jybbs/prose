def write_constant(out, value):
    if isinstance(value, (float, complex)):
        out.write(
            repr(value)
            .replace("infinity_and_beyond_and_more_and_more", "zzzzzzzzz")
            .replace("nan", "x")
        )
