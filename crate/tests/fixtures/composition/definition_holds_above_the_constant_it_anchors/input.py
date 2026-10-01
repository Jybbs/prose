class PackageFinder:
    find = None


class PEP420PackageFinder(PackageFinder):
    pass


find_packages = PackageFinder.find
find_namespace_packages = PEP420PackageFinder.find


def setup():
    pass


class Alpha:
    pass
