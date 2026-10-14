#!/usr/bin/env -S uv run --script
# MISE alias="labels"
# MISE confirm="Create or update every label .github/labels.toml declares on GitHub?"
# MISE description="Sync GitHub's labels to .github/labels.toml, listing those it omits"
# /// script
# requires-python = ">=3.11"
# ///
"""
Creates or updates every label `.github/labels.toml` declares on the
repository `gh` resolves from the working tree, then lists every live label
the registry omits, deleting none of them.

`gh label create --force` creates a missing label and updates the color
and description of one that exists, so a label renamed in the registry is
created as a new label beside the old one. The old label stays on GitHub and
is listed for a maintainer to rename or delete by hand.
"""

from pathlib    import Path
from subprocess import check_output, run
from tomllib    import loads


def main():
    """
    Creates or updates each label the registry declares, then lists every
    live label it omits.
    """
    labels = loads(Path(".github/labels.toml").read_text(encoding="utf-8"))["labels"]

    for name, label in labels.items():
        flags = ["--color", label["color"], "--description", label["description"]]
        run(["gh", "label", "create", name, *flags, "--force"], check=True)

    live = check_output(
        ["gh", "api", "--paginate", "repos/{owner}/{repo}/labels", "--jq", ".[].name"],
        text = True
    )

    if omitted := sorted(set(live.splitlines()) - labels.keys()):
        print("labels on GitHub that .github/labels.toml omits:")
        print("\n".join(f"  {name}" for name in omitted))


if __name__ == "__main__":

    main()
