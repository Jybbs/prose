"""
A custom Title-case section heading whose body carries `name: description`
entries reorders alphabetically by name, the same as the canonical Google
headings. The heading shape (*Title-case word, immediately followed by
`:`, body-indented children one step deeper*) is what qualifies the
section, not the heading's name.
"""


def run_pipeline(payload):
    """Run the data pipeline.

    Steps:
        validate: Reject malformed payloads at the boundary.
        ingest: Pull the payload into the staging table.
        normalize: Apply column-level transforms.
        emit: Hand the normalized rows to the downstream consumer.
    """
    return payload
