type LongerName = int
type Scored = Annotated[
    int, Field(ge=0)
]
