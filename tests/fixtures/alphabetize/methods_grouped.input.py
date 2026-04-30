"""
Methods within a class sort into four sub-groups in this order:
dunders, then properties (`@property` and `@cached_property`), then
private methods (single leading underscore), then public methods.
Each sub-group sorts alphabetically among its own members.
"""

class Matcher:
    def public_two(self):
        pass

    @property
    def cluster_count(self):
        pass

    def __init__(self, clusters):
        pass

    def _validate_two(self):
        pass

    @cached_property
    def adjacency(self):
        pass

    def __repr__(self):
        pass

    def public_one(self):
        pass

    def _validate_one(self):
        pass
