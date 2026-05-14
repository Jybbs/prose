"""
Dataclass declaration with annotated fields out of order, default
values of varying widths, and a method body below the field block.
The full pipeline sorts the field declarations, aligns the
annotation `:` and default `=` columns across the run, normalizes
the blank-line cushion between the field block and the method, and
reshapes the method's single-line docstring into multi-line form.
"""

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
