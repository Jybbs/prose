class Session:
    """
    Attributes:
        host      (str)  : The remote to dial.
        retry_budget (int) : How many attempts remain.
        cipher   (str): The negotiated suite.
    """

    host: str
    retry_budget: int
    cipher: str
