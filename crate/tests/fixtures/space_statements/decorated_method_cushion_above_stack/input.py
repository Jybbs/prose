class Posting:
    capacity: int = 0
    @cached_property
    @logged
    def label(self):
        return self.capacity
