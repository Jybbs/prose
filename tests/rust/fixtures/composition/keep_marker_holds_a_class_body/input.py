class StationSchema(DataFrameModel):  # prose: keep
    """Describes the station table."""
    station: Index[str] = Column(check_name=True)
    name: str = Column()
    latitude: float = Column()
