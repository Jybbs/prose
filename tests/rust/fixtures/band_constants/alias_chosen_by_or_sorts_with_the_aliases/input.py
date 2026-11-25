from _json import scanstring as c_scanstring
from json.scanner import py_scanstring
from os import sep

MAX_DEPTH = 32
scanstring = c_scanstring or py_scanstring
separator = sep or "/"
