class Loop:
    def _loop_self_reading(self):
        try:
            pass
        except BaseException as exc:
            self.call_exception_handler({
                'message': 'Error on reading from the event loop self pipe',
                'exception': exc,
                'loop': self,
            })
