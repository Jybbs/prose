#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Cut a draft release for `$VERSION`, or report when one already exists.

Probes `gh release view` for the version. A missing release is cut with
`gh release create --draft`, an existing draft is left untouched, and an
existing published release is skipped. Writes `state` (one of `cut`,
`drafted`, `published`) and the release `url` to `$GITHUB_OUTPUT`.
"""

from json       import loads
from os         import environ
from subprocess import run


def cut_draft(version: str, repo: str) -> str:
    """
    Create a draft release for the version and return its URL.
    """
    return run(
        [
            "gh", "release", "create", version, "--repo", repo,
            "--target", "main", "--generate-notes", "--draft"
        ],
        capture_output = True,
        check          = True,
        text           = True
    ).stdout.strip()


def existing_release(version: str, repo: str) -> dict | None:
    """
    Return the `isDraft`/`url` record for the version's release, or `None`
    when no release exists for it.
    """
    probe = run(
        ["gh", "release", "view", version, "--repo", repo, "--json", "isDraft,url"],
        capture_output = True,
        text           = True
    )
    return loads(probe.stdout) if probe.returncode == 0 else None


if __name__ == "__main__":

    version = environ["VERSION"]
    repo    = environ["GITHUB_REPOSITORY"]
    release = existing_release(version, repo)

    if release is None:
        state = "cut"
        url   = cut_draft(version, repo)
    elif release["isDraft"]:
        state = "drafted"
        url   = release["url"]
        print(f"::notice::Draft for {version} already exists, leaving it untouched.")
    else:
        state = "published"
        url   = release["url"]
        print(f"::warning::{version} is already published, skipping the draft cut.")

    with open(environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
        f.write(f"state={state}\n")
        f.write(f"url={url}\n")
