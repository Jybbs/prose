"""
The Prose binary a sweep formats with, meaning the rules it runs and the
diagnostic records a format run leaves.
"""

from functools  import cache
from json       import loads
from pathlib    import Path
from subprocess import check_output, run

EXIT_CAP = 4


def format_tree(
    binary : str,
    target : Path,
    select : str | None = None
) -> tuple[list[dict], str]:
    """
    Format `target` in place under every rule, or under `select` alone, and
    return the run's diagnostic records with its stderr.
    """
    chosen  = ["--select", select] if select else []
    command = [binary, "format", "--no-cache", "--output-format", "json", *chosen]
    done    = run([*command, str(target)], capture_output=True, cwd=target, text=True)
    if done.returncode > EXIT_CAP:
        raise SystemExit(
            f"format exited {done.returncode} on {target}: {last_line(done.stderr)}"
        )
    return [loads(line) for line in done.stdout.splitlines()], done.stderr


def last_line(text: str) -> str:
    """
    Return the last line `text` carries, or an empty string.
    """
    return text.strip().rpartition("\n")[2]


@cache
def rules(binary: str) -> list[str]:
    """
    Return the rule slugs `binary` runs, in pipeline order.
    """
    listed = check_output([binary, "rules", "--output-format", "json"])
    return [rule["slug"] for rule in loads(listed)]
