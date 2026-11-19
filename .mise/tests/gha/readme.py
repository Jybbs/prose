"""
Pins the paths `gha:readme` rewrites for PyPI, covering each form a README
cites a `site/` asset in, leaving every other relative path in place, and
the ref those paths pin to in a run that publishes a tag and one that does
not.
"""

from collections.abc import Callable
from pytest          import MonkeyPatch, mark, param
from types           import ModuleType

TAG = "0.11.1"
RAW = f"https://github.com/Jybbs/prose/raw/{TAG}/site/"


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
    assert task("gha/readme").absolute(before, TAG) == after


@mark.parametrize(
    ("environment", "ref"),
    [
        param(
            {"GITHUB_REF_NAME": TAG, "GITHUB_REF_TYPE": "tag"},
            TAG,
            id = "tag"
        ),
        param(
            {"GITHUB_REF_NAME": "12/merge", "GITHUB_REF_TYPE": "branch"},
            "main",
            id = "branch"
        ),
        param(
            {},
            "main",
            id = "local"
        )
    ]
)
def test_ref_pins_a_tag_run_to_its_tag_and_any_other_run_to_main(
    environment : dict[str, str],
    monkeypatch : MonkeyPatch,
    ref         : str,
    task        : Callable[[str], ModuleType]
):
    """
    Asserts a run a tag triggers pins the image addresses to that tag, while
    a branch run and a run outside GitHub pin them to `main`.
    """
    readme = task("gha/readme")
    monkeypatch.setattr(readme, "environ", environment)
    assert readme.ref() == ref
