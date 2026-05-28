"""
Single-item literals collapse to inline when their inline form
fits. The spec's `SETTINGS = {"default_action": "noop"}` case is
the most reduced. Empty literals spread across multiple lines
collapse to their bare-bracket form. A single-entry dict whose
entry overflows at its item-indent column breaks at `:` and hangs
the value at `item_indent + INDENT_STEP`, since any non-empty dict
qualifies as an expand target. Single-item lists and sets fall
outside expand-eligibility and so never enter the hang shape.
"""

only_list  = [42]
only_dict  = {"solitary": 1}
only_set   = {42}
empty_list = []
empty_dict = {}
collapsing_one_entry_dict = {
    "default_action": "noop"
}
collapsing_one_item_list = [
    42
]
collapsing_empty_dict = {
}
oversized_one_entry_dict = {
    "extremely_long_configuration_key_for_the_settings_table": "a_descriptive_value_long_enough_to_force_hang_shape"
}
