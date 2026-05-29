from dataclasses import dataclass


@dataclass
class JobPosting:
    title: str = "Untitled"
    salary_band: float = 0.0
    company: str = "TBD"
    description: str = ""
    posted_on: str = ""
    def summary(self):
        """Return a short summary string."""
        return f"{self.title} at {self.company}"
