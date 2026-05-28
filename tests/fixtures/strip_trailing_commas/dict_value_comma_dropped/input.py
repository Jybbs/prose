"""
A multi-line dict literal drops its trailing comma. The dict's
range covers `{...}`, and the backward scan from before the closing
`}` lands on the comma after the last value.
"""

config = {
    "linkage": "ward",
    "metric": "euclidean",
    "n_clusters": None,
    "threshold": 0.7,
}
