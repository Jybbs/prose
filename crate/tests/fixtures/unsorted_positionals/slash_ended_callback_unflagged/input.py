import sys


def lazily(importer, name, fromlist):
    return importer != "__main__"


def outside_tests(importer, name, fromlist, /):
    return not importer.startswith("tests.")


sys.set_lazy_imports_filter(lazily)
sys.set_lazy_imports_filter(outside_tests)
