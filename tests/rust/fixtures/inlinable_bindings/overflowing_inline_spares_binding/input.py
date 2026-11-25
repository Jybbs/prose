def archive(record, archive_root):
    digest = hashlib.sha256(record.encode()).hexdigest()
    return archive_root / "documents" / f"{digest}.json"
