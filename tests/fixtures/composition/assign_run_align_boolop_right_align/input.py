"""
A function body opens with a run of single-target assignments and
guards on a multi-line `BoolOp`. The assignment run aligns its `=`
column, and the `BoolOp`'s comparison operators right-align in their
own column.

Rules:
- align_equals
- align_comparisons
"""


def evaluate(record):
    name = record.get("name")
    user_id = record.get("user_id")
    age = record.get("age")
    if (
        name is None
        or user_id == 0
        or age < 18
    ):
        return None
    return record
