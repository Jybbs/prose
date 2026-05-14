"""
A long inline dict whose keys arrive out of order, with a trailing
comma in the source. The full dict-handling cycle expands the dict,
sorts the keys, strips the original trailing comma if it survives
the expansion, and aligns the `:` column across the rows.

Rules:
- collection_layout
- alphabetize
- strip_trailing_commas
- align_colons
"""

CONFIG = {"zeta": 6, "alpha_long": 1, "mango_label": 3, "beta_extended": 22, "delta_id": 44,}
