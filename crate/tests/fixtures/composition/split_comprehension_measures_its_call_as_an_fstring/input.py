class Policy:
    def __repr__(self):
        args = ["{}={!r}".format(name, value)
                for name, value in self.__dict__.items()]
        return args
