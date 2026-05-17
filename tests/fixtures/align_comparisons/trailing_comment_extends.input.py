"""
A trailing comment after the prev-operand source line does not
break the alignment run. The pipeline's `SuppressionMap` filter
handles directive comments separately on emitted edits.
"""

if (
    foo == 1  # trailing
    and bar_baz == 2
    and qux == 3
):
    pass
