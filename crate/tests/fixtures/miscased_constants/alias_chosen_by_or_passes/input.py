from _json import scanstring as c_scanstring


def py_scanstring(s, end):
    return s, end


scanstring = c_scanstring or py_scanstring
decoder    = c_decoder and py_decoder
encoding   = preferred_encoding or "utf-8"
