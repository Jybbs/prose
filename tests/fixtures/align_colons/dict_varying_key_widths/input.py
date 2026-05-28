"""
A multi-line dict literal with varying key widths. Every `:` aligns
to the column of the widest key, leaving zero padding on that row
and spaces before the `:` on the narrower rows.
"""

capitals = {
    "USA": "Washington",
    "France": "Paris",
    "Japan": "Tokyo",
    "Spain": "Madrid",
}
