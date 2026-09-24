class KeptStations(DataFrameModel):  # prose: keep
    """
    Describes the station table.

    Attributes:
        latitude: The station latitude.
        elevation: The station elevation, no longer a column.
        station: The station identifier.
        name: The station name.
    """

    station: Index[str] = Column(check_name=True)
    name: str = Column()
    latitude: float = Column()


class SortedStations(DataFrameModel):
    """
    Describes the station table.

    Attributes:
        latitude: The station latitude.
        elevation: The station elevation, no longer a column.
        station: The station identifier.
        name: The station name.
    """

    station: Index[str] = Column(check_name=True)
    name: str = Column()
    latitude: float = Column()
