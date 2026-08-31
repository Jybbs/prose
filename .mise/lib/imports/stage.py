"""
The scratch stage one sweep works in, meaning the original copy of the
corpus, each formatted copy, the overlays formatted under one rule, and the
home, records, and temporary directories the runs write to.
"""

from pathlib  import Path
from shutil   import copy2, copytree, ignore_patterns
from tempfile import mkdtemp


class Stage:
    """
    The scratch directory holding a corpus's original copy, its formatted
    copies, and the directories the runs write to.
    """

    def __init__(self, corpus: Path):
        self.corpus   = corpus
        self.root     = Path(mkdtemp(prefix="prose-imports."))
        self.home     = self.root / "home"
        self.records  = self.root / "records"
        self.tmp      = self.root / "tmp"
        self.original = self.copy("original")

        for directory in (self.home, self.records, self.tmp):
            directory.mkdir()

    def copy(self, name: str, width: int | None = None) -> Path:
        """
        Return a copy of the corpus at `name` under the root, without its
        bytecode caches, pinned at `width`.
        """
        tree = self.root / name
        copytree(self.corpus, tree, ignore=ignore_patterns("__pycache__"))
        configure(tree, width)

        return tree

    def overlay(
        self,
        files  : tuple[str, ...],
        label  : str,
        module : str,
        slug   : str,
        width  : int | None
    ) -> Path:
        """
        Return a tree holding the original of the top-level module or
        package carrying each of `files`, pinned at `width`, ready to
        be formatted under `slug` alone ahead of the original tree on
        `sys.path`.
        """
        tree = self.root / "alone" / label / module.replace("/", "+") / slug
        tree.mkdir(parents=True)

        for top in {file.split("/")[0] for file in files}:
            source, target = self.original / top, tree / top
            if source.is_dir():
                copytree(source, target)
            else:
                copy2(source, target)

        configure(tree, width)

        return tree


def configure(tree: Path, width: int | None):
    """
    Write the `prose.toml` pinning `width` at the root of `tree`, or nothing
    at the default width.
    """
    if width is not None:
        (tree / "prose.toml").write_text(
            f"code-line-length = {width}\n",
            encoding = "utf-8"
        )
