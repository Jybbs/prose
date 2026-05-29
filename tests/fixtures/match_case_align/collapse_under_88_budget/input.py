def dispatch(event):
    match event.kind:
        case "under_88_columns":
            counter = 1
        case "exactly_88_columns":
            counter = build(event.timestamp, event.src, event.k)
        case "longer_pattern_name":
            counter = build(event.timestamp, event.src, event.k)
        case "kind_with_descriptive_long_label":
            counter = build_for_long_kind(event.timestamp, event.source, event.kind)
        case "key" if some_long_predicate_check(event.kind):
            counter = compute(event.src)
