class Connection:
    def __init__(self, host, port, timeout):
        self.host = host


class SecureConnection(Connection):
    def __init__(self, host, port, timeout):
        super(SecureConnection, self).__init__(host,
                                               port,
                                               timeout)
