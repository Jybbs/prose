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
