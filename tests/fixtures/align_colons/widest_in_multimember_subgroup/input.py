"""
Class fields whose widths split into two sub-groups under the
rule's `max-shift` cap. The widest field sits in a multi-member
sub-group rather than its own singleton, so no anchor override
fires and each sub-group aligns at its own widest member's column.
"""

class Packet:
    host: str
    index: int
    payload_buffer: bytes
    response_buffer: list
