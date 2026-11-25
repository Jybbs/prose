def purge(paths, before):
    cutoff = before or 0.0
    for path in paths:
        if path.stat().st_mtime < cutoff:
            path.unlink()
