"""
Class fields whose widths span more than the rule's `max-shift`
apart trigger the default `split` policy. The greedy partitioner
produces three contiguous sub-groups, each aligning at its own
widest member's column. The outlier falls into a singleton
sub-group whose gap collapses to zero.
"""

class Packet:
    short: int
    medium_size: str
    really_really_long_field: bytes
    longer: int
    tiny: str
