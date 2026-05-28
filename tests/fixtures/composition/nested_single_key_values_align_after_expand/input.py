"""
A long single-line dict whose entries each carry a single-key nested
dict value. The outer dict overflows the inline budget, and after
expansion every nested single-key dict falls into a one-row colon
alignment group.

Rules:
- collection_layout
- alphabetize
- align_colons
"""

CONFIG = {"alpha": {"only_alpha_key": 1}, "beta": {"only_beta_key": 2}, "gamma": {"only_gamma_key": 3}, "delta": {"only_delta_key": 4}}
