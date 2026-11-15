"""
Pins the paths `gha:readme` rewrites for PyPI, covering each form a README
cites a `site/` asset in and leaving every other relative path in place.
"""

from collections.abc import Callable
from pytest          import mark
from types           import ModuleType

RAW = "https://github.com/Jybbs/prose/raw/main/site/"


@mark.parametrize(
    ("before", "after"),
    [
        ("![logo](../site/public/logo.svg)",          f"![logo]({RAW}public/logo.svg)"),
        ('<img src="../site/public/a.gif">',          f'<img src="{RAW}public/a.gif">'),
        (
            '<source srcset="../site/public/b.gif">',
            f'<source srcset="{RAW}public/b.gif">'
        ),
        ('<img src="../crate/a.svg">',                '<img src="../crate/a.svg">'),
        ("[guide](https://prose.fyi/guide/)",         "[guide](https://prose.fyi/guide/)")
    ]
)
def test_absolute_rewrites_each_site_path_form(
    after  : str,
    before : str,
    task   : Callable[[str], ModuleType]
):
    """
    Asserts each form citing a `site/` asset takes the raw URL and every
    other path stays as written.
    """
    assert task("gha/readme").absolute(before) == after
