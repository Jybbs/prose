def runner(invoke):
    def run(payload):
        result = invoke(payload)
        return result.returncode, loads(result.stdout) if result.stdout.strip() else None

    return run
