#!/usr/bin/env -S uv run --script
# MISE alias="rulesets"
# MISE confirm="Apply .github/rulesets and .github/settings.toml on GitHub?"
# MISE description="Apply .github/rulesets and .github/settings.toml to the repository"
# /// script
# requires-python = ">=3.11"
# ///
"""
Applies every ruleset under `.github/rulesets/` to the repository
`gh` resolves from the working tree, then sends every setting
`.github/settings.toml` declares.

A ruleset is matched to a live one by name, replaced through `PUT` where a
match exists and created through `POST` otherwise. A live ruleset no file
names is listed rather than deleted.

The settings file is read into a frozen record before anything is sent,
and an unknown or missing table or key, or a value of the wrong type, stops
the task. Each table takes the REST API's own names. `[repository]` is
the body of the repository `PATCH`, to which the task adds the description
from `crate/pyproject.toml` and the bare domain of its `homepage` URL. The
`keywords` there are sent as the topics. `[dependabot]` and `[advisories]`
hold one key per endpoint, `PUT` where it is `true` and `DELETE` where
it is `false`, and `[actions]` and `[workflow]` are the bodies of
`actions/permissions` and `actions/permissions/workflow`.
"""

from dataclasses  import asdict, dataclass, is_dataclass
from json         import dumps, loads
from pathlib      import Path
from subprocess   import check_output
from tomllib      import loads as from_toml
from typing       import Self, get_type_hints
from urllib.parse import urlsplit

PROJECT    = "crate/pyproject.toml"
REPOSITORY = "repos/{owner}/{repo}"
SETTINGS   = ".github/settings.toml"
RULESETS   = f"{REPOSITORY}/rulesets"


class Record:
    """
    Builds a frozen record from a table of the settings file, each field
    read from the key `spelled` names.
    """

    @classmethod
    def from_table(cls, table: object, path: str = "") -> Self:
        """
        Returns the record holding `table`, the table named `path` in the
        settings file, building each nested record from its own table. The
        task stops on a value that is not a table, a key the record does
        not declare, a key it declares that is missing, or a value of the
        wrong type.
        """
        hints = get_type_hints(cls)
        keys  = {cls.spelled(name): name for name in hints}
        where = f"[{path}]" if path else SETTINGS

        if not isinstance(table, dict):
            raise SystemExit(f"{where} is not a table")
        if unknown := sorted(table.keys() - keys.keys()):
            raise SystemExit(f"{where} declares unknown keys {unknown}")
        if missing := sorted(keys.keys() - table.keys()):
            raise SystemExit(f"{where} omits keys {missing}")

        values = {}
        for key, value in table.items():
            hint = hints[name := keys[key]]
            if is_dataclass(hint):
                value = hint.from_table(value, f"{path}.{key}" if path else key)
            elif not isinstance(value, hint):
                raise SystemExit(f"{where} key {key!r} is not a {hint.__name__}")
            values[name] = value
        return cls(**values)

    @staticmethod
    def spelled(name: str) -> str:
        """
        Returns the key the settings file spells the field `name` with.
        """
        return name


@dataclass(frozen=True)
class Actions(Record):
    """
    Holds the body of the `actions/permissions` call.
    """

    allowed_actions      : str
    enabled              : bool
    sha_pinning_required : bool


class Endpoints(Record):
    """
    Holds one flag per endpoint, each named for the endpoint's last path
    segment with underscores for hyphens.
    """

    def requests(self) -> list[tuple[str, str, None]]:
        """
        Returns the call toggling each endpoint, `PUT` where its flag is
        `true` and `DELETE` where it is `false`.
        """
        return [
            (f"{REPOSITORY}/{self.spelled(key)}", "PUT" if on else "DELETE", None)
            for key, on in asdict(self).items()
        ]

    @staticmethod
    def spelled(name: str) -> str:
        """
        Returns the endpoint's last path segment, the field `name` with
        hyphens for underscores.
        """
        return name.replace("_", "-")


@dataclass(frozen=True)
class Advisories(Endpoints):
    """
    Holds one flag per advisory endpoint.
    """

    private_vulnerability_reporting: bool


@dataclass(frozen=True)
class Dependabot(Endpoints):
    """
    Holds one flag per Dependabot endpoint in the order the calls are sent,
    the alerts first.
    """

    vulnerability_alerts     : bool  # prose: ignore[unsorted-positionals]
    automated_security_fixes : bool


@dataclass(frozen=True)
class Status(Record):
    """
    Holds the `status` of one `security_and_analysis` feature.
    """

    status: str


@dataclass(frozen=True)
class SecurityAndAnalysis(Record):
    """
    Holds the `security_and_analysis` features the repository call sets.
    """

    secret_scanning                 : Status
    secret_scanning_push_protection : Status


@dataclass(frozen=True)
class Repository(Record):
    """
    Holds the body of the repository `PATCH` the settings file declares.
    """

    allow_auto_merge       : bool
    allow_merge_commit     : bool
    allow_rebase_merge     : bool
    allow_squash_merge     : bool
    allow_update_branch    : bool
    delete_branch_on_merge : bool
    has_discussions        : bool
    has_issues             : bool
    has_projects           : bool
    has_wiki               : bool
    security_and_analysis  : SecurityAndAnalysis
    squash_merge_commit_message : str
    squash_merge_commit_title   : str
    web_commit_signoff_required : bool


@dataclass(frozen=True)
class Workflow(Record):
    """
    Holds the body of the `actions/permissions/workflow` call.
    """

    can_approve_pull_request_reviews : bool
    default_workflow_permissions     : str


@dataclass(frozen=True)
class Settings(Record):
    """
    Holds every table of the settings file.
    """

    actions    : Actions
    advisories : Advisories
    dependabot : Dependabot
    repository : Repository
    workflow   : Workflow

    def requests(self, project: dict) -> list[tuple[str, str, dict | None]]:
        """
        Returns the endpoint, method, and body of each call that applies
        these settings in the order they are sent, taking the description,
        the homepage's bare domain, and the topics from the `project` table
        of `crate/pyproject.toml`.
        """
        return [
            (
                REPOSITORY, "PATCH",
                {
                    **asdict(self.repository),
                    "description" : project["description"],
                    "homepage"    : urlsplit(project["urls"]["homepage"]).hostname
                }
            ),
            (f"{REPOSITORY}/topics", "PUT", {"names": project["keywords"]}),
            *self.dependabot.requests(),
            *self.advisories.requests(),
            (f"{REPOSITORY}/actions/permissions", "PUT", asdict(self.actions)),
            (f"{REPOSITORY}/actions/permissions/workflow", "PUT", asdict(self.workflow))
        ]


def gh(endpoint: str, *, body: dict | None = None, method: str = "GET") -> str:
    """
    Calls `endpoint` through `gh api` with `method`, sending `body` as JSON
    on standard input when one is given, and returns what it prints.
    """
    return check_output(
        [
            "gh", "api", "--method", method, endpoint,
            *(["--input", "-"] if body else [])
        ],
        input = dumps(body) if body else None,
        text  = True
    )


def main():
    """
    Reads the settings, applies the rulesets, and then sends the settings.
    """
    settings = Settings.from_table(from_toml(read(SETTINGS)))
    calls    = settings.requests(from_toml(read(PROJECT))["project"])

    live     = {ruleset["name"]: ruleset["id"] for ruleset in loads(gh(RULESETS))}
    recorded = set()

    for path in sorted(Path(".github/rulesets").glob("*.json")):
        ruleset = loads(read(path))
        recorded.add(name := ruleset["name"])
        if name in live:
            gh(f"{RULESETS}/{live[name]}", body=ruleset, method="PUT")
            print(f"updated {name!r} from {path}")
        else:
            gh(RULESETS, body=ruleset, method="POST")
            print(f"created {name!r} from {path}")

    for endpoint, method, body in calls:
        gh(endpoint, body=body, method=method)
    print(f"repository settings applied from {SETTINGS}")

    if omitted := sorted(live.keys() - recorded):
        print("rulesets on GitHub that .github/rulesets omits:")
        print("\n".join(f"  {name}" for name in omitted))


def read(path: str | Path) -> str:
    """
    Returns the text of the file at `path`.
    """
    return Path(path).read_text(encoding="utf-8")


if __name__ == "__main__":

    main()
