class Waiter:
    def start(self, loop, timeout):
        self._timeout_handle = (
            loop.call_later(timeout, self._handle_timeout)
        )
