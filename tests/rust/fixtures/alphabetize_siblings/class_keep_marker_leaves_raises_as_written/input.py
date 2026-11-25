class Stations:  # prose: keep
    """
    Loads the station table.

    Attributes:
        name: The station name.
        station: The station identifier.

    Raises:
        ValueError: The table holds no station.
        KeyError: A station repeats.
    """

    station: str
    name: str
