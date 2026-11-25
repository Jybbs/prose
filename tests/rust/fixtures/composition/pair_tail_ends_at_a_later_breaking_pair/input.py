class peekable:
    def _get_slice(self, index):
        if step < 0:
            stop = (-maxsize - 1), (index.stop is None and step)
