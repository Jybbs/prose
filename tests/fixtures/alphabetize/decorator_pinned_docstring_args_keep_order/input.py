@pytest.mark.parametrize("value", [1])
def normalize(target, source):
    """Apply ``source`` onto ``target``.

    Args:
        target: Mapping receiving the update.
        source: Mapping providing new values.
    """
