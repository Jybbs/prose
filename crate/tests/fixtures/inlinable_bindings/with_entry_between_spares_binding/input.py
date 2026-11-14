def snapshot(lock, ledger, entry):
    balance = ledger.balance
    with lock:
        latest = entry
    return balance, latest
