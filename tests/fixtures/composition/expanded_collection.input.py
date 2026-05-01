"""
Long dict literal that fits on one line in source but overflows the
line-length budget once expanded. collection_layout puts each entry on
its own line and recursively expands the long nested dict, alphabetize
partitions single-line entries before the multi-line entry and sorts
within each partition, align_colons aligns the colons across the
entries, and strip_trailing_commas removes any dangling commas the
source carried. The full pipeline running twice on this input produces
no further change.
"""

CONFIG = {"timeout": 30, "retries": 5, "backoff": 1.5, "verbose": True, "label": "primary-region-default", "tags": ["staging", "ingest"], "limits": {"requests_per_minute": 600, "burst_capacity": 1200, "concurrent_streams": 8, "queue_depth": 256}}
