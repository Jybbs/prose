def collect(records, detailed):
    names = list(record.name for record in records) if detailed else the_default_names_list
