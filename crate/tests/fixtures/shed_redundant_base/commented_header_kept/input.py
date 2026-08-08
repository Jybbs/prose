class Annotated(
    object,  # spelled out for the reader
):
    def note(self):
        return self.comment


class Bare(  # no bases at all
):
    def note(self):
        return None
