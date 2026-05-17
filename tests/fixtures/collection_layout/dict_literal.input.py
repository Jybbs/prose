"""
A multi-item dict literal stays inline when the whole line fits in
the 88-character budget. A dict whose inline form would overflow the
budget expands with each `key: value` entry on its own line. A
multi-line dict whose assembled inline form fits collapses back to
one line, the single-entry dict being the most reduced canonical
case. Dict items are never flow-packed regardless of how simple the
key and value are, because the pair itself carries structure and
downstream `align_colons` needs rows to align across.
"""

short_dict = {"alpha": 1, "beta": 2, "gamma": 3}
long_dict = {"alpha": 1, "beta": 2, "gamma": 3, "delta": 4, "epsilon": 5, "zeta": 6, "eta": 7}
single_entry_dict = {
    "default_action": "noop"
}
collapsing_dict = {
    "alpha": 1,
    "beta": 2,
    "gamma": 3
}
