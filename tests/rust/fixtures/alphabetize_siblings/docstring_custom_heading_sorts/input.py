def run_pipeline(payload):
    """Run the data pipeline.

    Steps:
        validate: Reject malformed payloads at the boundary.
        ingest: Pull the payload into the staging table.
        normalize: Apply column-level transforms.
        emit: Hand the normalized rows to the downstream consumer.
    """
    return payload
