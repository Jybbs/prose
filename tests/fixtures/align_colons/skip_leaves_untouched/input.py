"""
Under the `skip` policy, a class-field group whose widest name
exceeds the rule's `max-shift` stays entirely untouched. No
member's pre-colon whitespace changes, outlier and neighbors alike.
"""

class Packet:
    short: int
    medium_size: str
    really_really_long_field: bytes
    longer: int
    tiny: str
