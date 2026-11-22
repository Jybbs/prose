filtered = [
    transform_each_record(record)
    for record in the_complete_source_collection
    if record.is_currently_active
]
