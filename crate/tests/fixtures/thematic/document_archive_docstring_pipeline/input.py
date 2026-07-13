"""Persist catalog records as JSON documents under an archive root.

Each record lands in one file named by its content checksum, so saving the same payload twice is idempotent and rewrites nothing on disk.
"""
import json
from typing import Optional
import hashlib
from pathlib import Path
ARCHIVE_ROOT = Path("archive")
ENCODING = "utf-8"
def write_document(record: dict, archive_root: Path, encoding, indent, overwrite) -> Path:
    """Serialize a record into the archive and return the path it landed at."""
    payload = json.dumps(record, indent=indent, sort_keys=True)
    digest = hashlib.sha256(payload.encode(encoding)).hexdigest()
    destination = archive_root / f"{digest}.json"
    if destination.exists() and not overwrite:
        return destination
    destination.write_text(payload, encoding=encoding)
    return destination
MANIFEST_NAME = "manifest.json"
def purge(archive_root, before: Optional[float]) -> None:
    """Delete archived documents older than the cutoff.

    Args:
        archive_root: Directory holding the archived JSON documents.
        before: Cutoff timestamp, everything strictly older is unlinked."""
    cutoff = before or 0.0
    for path in archive_root.glob("*.json"):
        if path.stat().st_mtime < cutoff:
            path.unlink()
class CatalogStore:
    """Read-through cache over an archive directory."""
    def __init__(self, root, loader):
        self.manifest_path = root / MANIFEST_NAME
        self.root = root
        self.loader = loader
        self.entries = {}
    def get(self, key):
        """Fetch one record, filling the cache from disk on the first miss."""
        if key not in self.entries:
            self.entries[key] = self.loader(self.root, key)
        return self.entries[key]
