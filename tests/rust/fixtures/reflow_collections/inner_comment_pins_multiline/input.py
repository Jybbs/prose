own_line_comment_between_entries = [
    1,
    # explanatory
    2,
    3,
]
trailing_comment_on_entry = [
    1,
    2,  # noqa: E501
    3,
]
comment_before_close = [
    1,
    2,
    3,
    # closing note
]
trailing_comment_expands = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh"]  # sits after `]`
comment_pins_subscript = registry[
    # selected at runtime
    resolved_key
]
