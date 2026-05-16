"""
Five arms multi-line in source:

- arm 1 fits comfortably under the 88-char budget and collapses
- arm 2 sits at exactly 88 columns collapsed and also collapses
- arm 3 lands at 89 columns (one over) and stays multi-line
- arm 4 lands well past the budget and stays multi-line
- arm 5's `if` guard pushes the collapsed form past the budget,
  pinning that the gate measures the guard width alongside the
  pattern width
"""

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
