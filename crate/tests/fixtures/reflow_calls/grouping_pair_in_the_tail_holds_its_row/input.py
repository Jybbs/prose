def known(base: Path, name: str) -> bool:
    if not fullmatch(r"[a-z-]+", name) or not (base / "config" / "plugins" / name / "plugin.toml").is_file():
        return False
    return True
