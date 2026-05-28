"""
Under the `drop` policy, a class-field group whose widest name
exceeds the rule's `max-shift` excludes the outlier from the
alignment math. The remaining fields align against one shared `:`
column as if the outlier were invisible, and the outlier itself
keeps its original spacing.
"""

class Packet:
    short: int
    medium_size: str
    really_really_long_field: bytes
    longer: int
    tiny: str
