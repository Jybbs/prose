class Loop:
    def _loop_self_reading(self):
        try:
            pass
        except BaseException as exc:
            context = {
                'message': 'Error on reading from the event loop self pipe',
                'exception': exc,
                'loop': self,
            }
            self.call_exception_handler(context['message'], context['loop'])
