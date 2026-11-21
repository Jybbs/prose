"""
Pins the arguments `site:affected` hands `vitest`.
"""

from collections.abc import Callable
from pytest          import mark, param
from subprocess      import CompletedProcess

VITEST = [
    "vitest", "run", "--changed", "origin/main",
    "--config", ".vitepress/vitest.config.ts", "--project", "docs"
]


@mark.parametrize(
    "args",
    [param((), id = "no arguments"), param(("--reporter", "dot"), id = "forwarded")]
)
def test_affected_hands_vitest_the_docs_project_and_origin_main(
    affected : Callable[..., CompletedProcess],
    args     : tuple[str, ...]
):
    """
    Pins that the task hands `vitest run` the `docs` project, the site's
    `vitest` config, and `--changed origin/main`, with the arguments it is
    given after its own.
    """
    finished = affected("site/index.md", "edited", *args, group="site")

    assert finished.returncode == 0
    assert finished.stdout.split() == [*VITEST, *args]
