"""
Class body with annotated fields followed by methods, then more
fields after the methods. The methods break the field run into two
independent groups. Each group aligns its `:` column at its own
widest-key width, not as a single cross-method group.
"""

class Account:
    id: int
    user_name: str
    email: str

    def activate(self) -> None:
        self.is_active = True

    def deactivate(self) -> None:
        self.is_active = False

    balance: float
    currency_code: str
