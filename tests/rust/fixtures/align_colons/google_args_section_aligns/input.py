def transfer(source: str, destination: str, amount: float, memo: str) -> None:
    """
    Transfers funds between two accounts.

    Args:
        source: account id to debit from.
        destination: account id to credit to.
        amount: quantity in the account's base currency.
        memo: optional note that accompanies the transfer.

    Returns:
        None.
    """
    _debit(source, amount)
    _credit(destination, amount, memo)
