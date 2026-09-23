class Pipeline:
    name: str
    retry_budget: int

    def run(self, steps, verbose, destination):
        """
        Args:
            steps       : The names of the steps to compute.
            verbose     : Whether to print the records.
            destination : Where the results land.
        """
