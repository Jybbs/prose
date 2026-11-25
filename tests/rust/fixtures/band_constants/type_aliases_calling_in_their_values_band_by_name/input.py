type Meters = Annotated[int, Field(ge=0)]

type Latitude = Annotated[float, Field(ge=-90, le=90)]

type Names = Annotated[list[Identifier], AfterValidator(ordered)]

type Identifier = Annotated[str, StringConstraints(pattern=r"^[A-Za-z0-9_.]+$")]


def ordered(names: list[str]) -> list[str]:
    return sorted(names)
