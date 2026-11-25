def load_lexicon(
    path: Path,
    encoding: str,
    fallback: dict[str, str],
) -> LexiconLoader:
    return LexiconLoader(path, encoding, fallback)
