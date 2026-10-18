def latest(transcript, moment):
    stamps = []
    if transcript:
        for path in transcript.parent.glob("*.jsonl"):
            with suppress(OSError):
                written = path.stat().st_mtime
                if path != transcript and abs(written - moment) <= RECENT.total_seconds():
                    stamps.append((written, path))
    return max(stamps, default=(0.0, None))[1]
