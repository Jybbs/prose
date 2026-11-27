"""
Pins that each `[doctor.checks]` entry of `.mise/config.toml` that cannot hold
on a CI runner passes there, and fails where its condition does apply.
"""

from pathlib    import Path
from pytest     import mark, param
from subprocess import run
from tomllib    import loads

CHECKS = loads(
    (Path(__file__).resolve().parents[3] / ".mise/config.toml").read_text()
)["doctor"]["checks"]
PATH   = "/usr/bin:/bin"


@mark.parametrize(
    ("check", "env", "code"),
    [
        param("clone", {"CI": "true"}, 0, id = "a one-commit clone on CI"),
        param("clone", {}, 1, id = "no repository off CI"),
        param("token", {"CI": "true"}, 0, id = "no token on CI"),
        param("token", {}, 127, id = "no token off CI"),
        param("venv", {}, 0, id = "no virtualenv loaded")
    ]
)
def test_a_check_passes_only_where_its_condition_applies(
    check    : str,
    code     : int,
    env      : dict[str, str],
    tmp_path : Path
):
    """
    Runs the check in an empty directory holding no repository, with no
    `mise` on `PATH`, so a check whose condition applies fails.
    """
    result = run(
        ["sh", "-c", CHECKS[check]["run"]],
        capture_output = True,
        cwd            = tmp_path,
        env            = {"PATH": PATH, **env}
    )
    assert result.returncode == code
