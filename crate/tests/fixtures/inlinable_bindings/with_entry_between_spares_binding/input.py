def snapshot(lock, ledger):
    balance = ledger.balance
    with lock:
        return balance
