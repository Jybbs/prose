#!/usr/bin/env -S uv run --script
# MISE description="Create or refresh the draft release for the crate's version"
# /// script
# requires-python = ">=3.11"
# ///
"""
Keeps one draft release for the version `crate/Cargo.toml` carries.

Reads `[package].version`, emits it, then reads every page of the
repository's releases through `gh api` and takes one outcome from the
release carrying that version. A version no release carries is created as
a draft with generated notes. A draft carrying it has its body replaced
with the notes the `releases/generate-notes` endpoint returns for the
version against `main`, sent with the version as `tag_name` so the draft
keeps it. A draft named for the version whose tag reads `untagged-<hash>`
counts as carrying it. A published release carrying it stops the task with
an `::error::` annotation naming `mise bump`, so no merge adds to a version
already released. A `gh` call that exits nonzero stops the task with an
`::error::` annotation carrying every line it printed on stderr, so no draft
is written after a failed read and a rerun retries it. Emits `version=`,
then `state=` (one of `created`, `refreshed`, or `published`) and `url=`,
to `$GITHUB_OUTPUT` when it is set, stdout otherwise, with annotations on
stderr.
"""

from itertools  import chain
from json       import loads as from_json
from os         import environ
from pathlib    import Path
from subprocess import CalledProcessError, PIPE, check_output
from tomllib    import loads as from_toml

ESCAPES  = str.maketrans({"%": "%25", "\n": "%0A", "\r": "%0D"})
MANIFEST = "crate/Cargo.toml"


def annotation(level: str, message: str) -> str:
    """
    Formats the workflow command raising a `level` annotation for
    `message`, with each `%`, carriage return, and newline escaped the way
    `@actions/core` escapes a command's message, which the runner reverses
    when it reads the annotation.
    """
    return f"::{level}::{message.translate(ESCAPES)}"


def create_draft(repo: str, version: str) -> str:
    """
    Creates a draft release for the version with generated notes and returns
    its URL.
    """
    return gh(
        "release",
        "create",
        version,
        "--repo",
        repo,
        "--target",
        "main",
        "--generate-notes",
        "--draft"
    ).strip()


def emit(**outputs: str):
    """
    Appends one `key=value` line per output to `$GITHUB_OUTPUT`, or prints
    the lines when it is unset.
    """
    lines = [f"{key}={value}" for key, value in outputs.items()]
    if path := environ.get("GITHUB_OUTPUT"):
        with open(path, "a", encoding="utf-8") as f:
            print(*lines, file=f, sep="\n")
    else:
        print(*lines, sep="\n")


def existing_release(repo: str, version: str) -> dict | None:
    """
    Finds the first release in the repository's listing whose tag is the
    version, else the first draft named for it whose tag a refresh once
    replaced with `untagged-<hash>`, giving `None` when neither exists. A
    tag match wins wherever it sits, so a published release carrying the
    version still stops the task ahead of a stale untagged draft.
    """
    listing  = gh("api", "--paginate", "--slurp", f"repos/{repo}/releases")
    releases = list(chain.from_iterable(from_json(listing)))
    return next(
        (release for release in releases if release["tag_name"] == version),
        next(
            (
                release for release in releases
                if release["draft"] and release["name"] == version
            ),
            None
        )
    )


def gh(*args: str) -> str:
    """
    Runs `gh` with `args` and returns what it prints, stopping the task
    with an `::error::` annotation carrying every line of its stderr when it
    exits nonzero.
    """
    try:
        return check_output(["gh", *args], stderr=PIPE, text=True)
    except CalledProcessError as error:
        raise SystemExit(annotation("error", error.stderr.strip()))


def main():
    """
    Emits the manifest's version, then creates, refreshes, or refuses the
    version's release by what the listing holds, and emits the outcome.
    """
    repo    = environ["GITHUB_REPOSITORY"]
    version = package_version(Path(MANIFEST).read_text(encoding="utf-8"))

    emit(version=version)
    release = existing_release(repo, version)

    if release is None:
        emit(state="created", url=create_draft(repo, version))
    elif release["draft"]:
        emit(state="refreshed", url=refresh_draft(release, repo, version))
    else:
        emit(state="published", url=release["html_url"])
        raise SystemExit(
            annotation(
                level   = "error",
                message = f"{version} is already published. Run `mise bump` to "
                          f"move {MANIFEST} to the next version."
            )
        )


def package_version(manifest: str) -> str:
    """
    Reads `[package].version` from the text of a `crate/Cargo.toml`.
    """
    return from_toml(manifest)["package"]["version"]


def refresh_draft(release: dict, repo: str, version: str) -> str:
    """
    Replaces the draft's body with the notes GitHub generates for the
    version against `main`, sending the version as its `tag_name` and `main`
    as its `target_commitish` so the edit leaves the draft tagged with the
    version, and returns the URL GitHub gives the draft after the edit.
    """
    notes = gh(
        "api",
        "--method",
        "POST",
        f"repos/{repo}/releases/generate-notes",
        "-f",
        f"tag_name={version}",
        "-f",
        "target_commitish=main"
    )
    edited = gh(
        "api",
        "--method",
        "PATCH",
        f"repos/{repo}/releases/{release['id']}",
        "-f",
        f"tag_name={version}",
        "-f",
        "target_commitish=main",
        "-f",
        f"body={from_json(notes)['body']}"
    )
    return from_json(edited)["html_url"]


if __name__ == "__main__":

    main()
