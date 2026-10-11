"""
Pins the `gh` calls `repo:labels` sends for the labels the registry
declares, and that it lists each live label the registry omits rather than
deleting it.
"""

from collections.abc   import Callable
from pathlib           import Path
from pytest            import CaptureFixture, fixture
from pytest_subprocess import FakeProcess
from types             import ModuleType

CREATED = [
    ["gh", "label", "create", name, "--color", color, "--description", text, "--force"]
    for name, color, text in [
        ("🐞 bug", "d73a4a", "A defect in what the formatter writes"),
        ("🗜️ build", "e6e6ff", "Cargo.toml, maturin, and dependencies")
    ]
]
LIVE = ["gh", "api", "--paginate", "repos/{owner}/{repo}/labels", "--jq", ".[].name"]
REGISTRY = """
[labels."🐞 bug"]
color       = "d73a4a"
description = "A defect in what the formatter writes"

[labels."🗜️ build"]
color       = "e6e6ff"
description = "Cargo.toml, maturin, and dependencies"
"""


@fixture
def labels(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Returns the loaded `repo:labels` script.
    """
    return task("repo/labels")


def test_main_creates_every_label_then_lists_the_omitted(
    capsys  : CaptureFixture[str],
    fp      : FakeProcess,
    labels  : ModuleType,
    workdir : Callable[[dict[str, str]], Path]
):
    """
    Pins that each registry label is created or updated through one call
    carrying its color and description with `--force`, in the order the
    registry declares them, and that a live label the registry omits is
    listed and left in place, `fp` raising on any other command.
    """
    workdir({".github/labels.toml": REGISTRY})
    for command in CREATED:
        fp.register(command)
    fp.register(LIVE, stdout="🐞 bug\nstray\n🗜️ build\n")

    labels.main()

    assert list(fp.calls) == [*CREATED, LIVE]
    assert capsys.readouterr().out == (
        "labels on GitHub that .github/labels.toml omits:\n  stray\n"
    )
