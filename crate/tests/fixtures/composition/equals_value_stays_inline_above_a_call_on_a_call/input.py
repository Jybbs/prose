body["__module__"] = module
tmp_cls = type(name, (object,), body)
cls = _simple_enum(boundary=boundary or KEEP, etype=cls)(tmp_cls)
