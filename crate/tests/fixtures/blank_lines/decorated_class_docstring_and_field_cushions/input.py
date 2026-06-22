@final
@dataclass
class Posting:

    """
    Hold a single posting's normalized fields.
    """
    company: str
    title: str
    def summary(self):
        return f"{self.title} at {self.company}"
