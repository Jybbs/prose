class Configured(metaclass=ConfiguredMeta):
    def configure(self):
        return self.options


class Dynamic(*RESOLVED_BASES):
    def resolve(self):
        return self.target


class Forwarded(**NAMESPACE):
    def forward(self):
        return self.inner


class Plain:
    def plain(self):
        return None
