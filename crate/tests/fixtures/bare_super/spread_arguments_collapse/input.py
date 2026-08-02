class Handler:
    def dispatch(self, event):
        return event


class Logger(Handler):
    def dispatch(self, event):
        return super(
            Logger,
            self
        ).dispatch(event)
