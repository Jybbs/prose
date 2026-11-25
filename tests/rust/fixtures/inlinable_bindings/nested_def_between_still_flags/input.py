def schedule(jobs):
    count = len(jobs)

    def run():
        jobs.clear()

    return count, run
