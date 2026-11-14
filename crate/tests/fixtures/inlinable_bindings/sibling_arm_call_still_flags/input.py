def settle(ledger, closed):
    balance = ledger.total
    if closed:
        ledger.archive()
    else:
        return balance
