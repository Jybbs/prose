class Pipeline:
    """
    Attributes:
        name (str): The label on each record.
        retry_budget (int): How many attempts remain.
    """

    def run(self, steps, verbose):
        """
        Args:
            steps: The names of the steps to compute.
            verbose: Whether to print the records.
        """
