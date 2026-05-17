"""
A class field directly followed by a decorated method carries 1 blank
line of cushion. The cushion sits above the topmost decorator rather
than between the decorator and the def, so the decorator stack reads
as bound to the method below it.
"""


class Posting:
    capacity: int = 0
    @cached_property
    @logged
    def label(self):
        return self.capacity
