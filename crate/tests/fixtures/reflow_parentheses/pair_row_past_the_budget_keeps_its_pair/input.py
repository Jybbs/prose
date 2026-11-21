class Waiter:
    def start(self, loop, timeout):
        self._timeout_handle_of_the_waiter = (
            loop.call_later(timeout, self._handle_timeout)
        )
