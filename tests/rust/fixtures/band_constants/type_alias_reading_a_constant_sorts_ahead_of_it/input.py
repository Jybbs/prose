CEILING = 90
type Bounded = Annotated[int, Field(le=CEILING)]
