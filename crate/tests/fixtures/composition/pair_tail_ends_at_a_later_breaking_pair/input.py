class peekable:
    def _get_slice(self, index):
        if step < 0:
            stop = (-maxsize - 1) if (index.stop is None) else index.stop
