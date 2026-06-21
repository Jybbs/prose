config = {
    "alpha": 1,
    "beta": 2,
    "extremely_long_key_name_that_pushes_its_row_past_the_budget": "value_long_enough_to_overflow_at_item_indent"
}
mixed = {
    "alpha": 1,
    "extremely_long_key_name_that_pushes_its_row_past_the_budget": "value_long_enough_to_overflow_at_item_indent",
    "gamma": 3
}
nested_value_passes_through = {
    "alpha": 1,
    "limits": {"burst_capacity": 1200, "concurrent_streams": 8, "queue_depth": 256, "requests_per_minute": 600},
    "gamma": 3
}
multiple_hung_rows = {
    "alpha": 1,
    "beta": 2,
    "extremely_long_key_name_that_pushes_its_row_past_the_budget": "value_long_enough_to_overflow_at_item_indent",
    "z_another_extremely_long_key_pushing_past_the_budget": "another_value_long_enough_to_overflow_at_item_indent"
}
