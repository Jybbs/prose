class KeptStations(DataFrameModel):  # prose: keep
    """
    Describes the station table.

    Attributes:
        station: The station identifier.
        name: The station name.
        latitude: The station latitude.
    """

    station: Index[str] = Column(check_name=True)
    name: str = Column()
    latitude: float = Column()


class SortedStations(DataFrameModel):
    """
    Describes the station table.

    Attributes:
        station: The station identifier.
        name: The station name.
        latitude: The station latitude.
    """

    station: Index[str] = Column(check_name=True)
    name: str = Column()
    latitude: float = Column()
