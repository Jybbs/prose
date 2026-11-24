"""
Pins that the baseline `gha:baseline` bakes in place and the head
`rust:delta` then formats with each come from a build of their own.
"""

from collections.abc import Callable
from pathlib         import Path
from subprocess      import CompletedProcess, run


def test_rebake_leaves_the_head_a_build_of_its_own(
    delta    : Callable[..., CompletedProcess],
    tmp_path : Path
):
    """
    Pins that a head formatting a module differently from its parent shows
    that module after the row bakes the parent's stage in place.
    """
    stage  = str(tmp_path / "stage")
    parent = run(
        ["git", "rev-parse", "main"],
        capture_output = True,
        check          = True,
        cwd            = tmp_path / "clone",
        text           = True
    ).stdout.strip()
    baked    = delta("gha/baseline", PARENT=parent, STAGE=stage)
    reported = delta("rust/delta", PROSE_DELTA_STAGE=stage)

    assert baked.returncode == reported.returncode == 0
    assert "module.py" in reported.stdout.splitlines()
