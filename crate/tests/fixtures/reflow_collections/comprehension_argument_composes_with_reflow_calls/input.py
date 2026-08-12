configure(
    alpha=1,
    beta=2,
    gamma=3,
    items=[
        value
        for value in source
    ],
)
register(
    alpha=1,
    beta=2,
    gamma=3,
    rows=[
        transform_each_row(row)
        for row in the_complete_collection_of_source_rows
        if row.is_eligible
    ],
)
