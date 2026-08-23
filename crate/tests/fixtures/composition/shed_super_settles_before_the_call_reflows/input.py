class Waiter(Base):
    def add_exception(self, future):
        with self.lock:
            super(Waiter, self).add_exception(future)
