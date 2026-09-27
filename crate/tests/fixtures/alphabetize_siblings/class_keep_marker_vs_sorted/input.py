class KeptStations(DataFrameModel):  # prose: keep
    station  : Index[str] = Column(check_name=True)
    name     : str        = Column()
    latitude : float      = Column()


class SortedStations(DataFrameModel):
    station  : Index[str] = Column(check_name=True)
    name     : str        = Column()
    latitude : float      = Column()
