class Adapter(
    Mapping,
    object,
):
    def get(self, key):
        return self.data[key]


class Wrapper(
    object,
    metaclass=WrapperMeta,
):
    def unwrap(self):
        return self.inner
