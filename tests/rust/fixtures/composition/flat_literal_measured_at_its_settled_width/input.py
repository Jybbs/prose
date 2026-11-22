class Logger:
    def __init__(self, alogger):
        self.loggerMap = { alogger : None }
        m = {"nw":1, "sw":2, "ne":3, "se":4}
