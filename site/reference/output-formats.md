---
description: "Covers the `text`, `json`, `github`, and `sarif` formats."
---

# Output Formats

`--output-format` selects the format diagnostics print in, with one format per kind of reader. `text` is the default for a person reading a terminal, printing rustc-style snippets with carets and a suggested fix. `json` prints Ruff-shaped NDJSON for editor plugins and tooling. `github` prints workflow commands that <Tool slug="github" /> renders as inline annotations. `sarif` prints a [**SARIF 2.1.0**](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) document for upload to [**GitHub Code Scanning**](https://docs.github.com/en/code-security/code-scanning).

Every format writes diagnostics to stdout, and operational errors *(parse failures, IO errors, config errors)* go to stderr with an `error:` prefix, so a CI pipeline can separate the two streams without reading exit codes. A file whose syntax its `target-version` rejects prints its `syntax error` line on stderr as well, naming the file, the position, and the version, and the `json`, `github`, and `sarif` formats also print it as a record at that position, so a reader positions it the way it positions a diagnostic.

::: warning Diff Mode Is Text-Only
`--diff` needs `text`, as the [**Mutual Exclusion**](/reference/cli#mutual-exclusion) section of the CLI reference covers.
:::

## `text`

The default format prints each diagnostic as a rustc-style snippet, where a caret marks the range the finding covers, a label names the rule, and, for a diagnostic carrying a fix, a HELP block shows the replacement.

```
warning: align consecutive `=` operators
  --> src/module.py:14:8
   |
14 |     foo = 1
   |        ^ align-equals
   |
help: replace with
   |
14 |     foo   = 1
   |         ++
```

A notebook diagnostic prints under a `cell N` header against that cell's own source, with the caret positioned within the cell.

The snippets are rendered by [**`annotate-snippets`**](https://docs.rs/annotate-snippets/) and the color by [**`anstream`**](https://docs.rs/anstream/), so the global `--color` flag controls the ANSI sequences.

## `json`

One Ruff-compatible record per diagnostic or refused file on its own line, in NDJSON, closed by a summary record. The record shape matches what Ruff prints, so an editor with a diagnostics panel shows each record as an inline squiggle and uses the `fix` payload for a quick-fix action. Every record opens with a `kind` field, `"diagnostic"` for a finding, `"syntax_error"` for a file its `target-version` refuses, and `"summary"` for the closing record, so a streaming reader can dispatch on the first parsed property without inferring the kind from which fields are present.

```json
{
  "kind"         : "diagnostic",
  "code"         : "align-equals",
  "end_location" : { "column": 9, "row": 14 },
  "filename"     : "src/module.py",
  "fix"          : {
    "applicability" : "safe",
    "edits"         : [
      {
        "before"       : " ",
        "content"      : "   ",
        "end_location" : { "column": 9, "row": 14 },
        "location"     : { "column": 8, "row": 14 }
      }
    ]
  },
  "location"     : { "column": 8, "row": 14 },
  "message"      : "align consecutive `=` operators"
}
```

Fields:

| Field | Type | Meaning |
|---|---|---|
| `kind` | `"diagnostic"` | Record kind, always the first key |
| `code` | string | The rule's [[rule-id]] slug |
| `filename` | string | Source path |
| `cell` | number | *Notebook only.* The one-indexed cell holding the finding, absent for a module |
| `location` | `{ row, column }` | One-indexed start position |
| `end_location` | `{ row, column }` | One-indexed end position |
| `message` | string | The finding, in the rule's words |
| `fix` | object \| null | `null` for a lint finding carrying no suggestion, otherwise `{ applicability, edits }` |
| `fix.applicability` | `"safe"` \| `"displayonly"` | Confidence that the edits preserve runtime semantics |
| `fix.edits` | array of `{ before, content, location, end_location }` | The replacement spans an editor or CI can apply |
| `fix.edits[].before` | string | The original text at the edit's range, paired with `content` for a before and after view without reading the source again |

For a notebook input, `location` and `end_location` are positions within the cell `cell` names rather than offsets into the joined source, whereas a module input's positions are absolute.

`applicability` is `"safe"` for every auto-fix *Prose* prints, on the scale Ruff uses, where `safe` means the rewrite preserves runtime semantics and an editor can apply it without asking. A lint finding carrying a suggestion (*the SCREAMING_CASE rename `miscased-constants` offers, the annotation `signature-annotations` infers*) prints `"displayonly"`, so an editor shows the edit and never applies it on its own.

An auto-fix that touches several lines prints one diagnostic holding all its edits in `fix.edits`. Three consecutive assignments under `align-equals` print one entry for each row it pads, since `backoff_base` already sits at the column:

```json
{
  "kind"         : "diagnostic",
  "code"         : "align-equals",
  "end_location" : { "column": 13, "row": 13 },
  "filename"     : "src/configure.py",
  "fix"          : {
    "applicability" : "safe",
    "edits"         : [
      {
        "before"       : " ",
        "content"      : "      ",
        "end_location" : { "column": 13, "row": 12 },
        "location"     : { "column": 12, "row": 12 }
      },
      {
        "before"       : " ",
        "content"      : "      ",
        "end_location" : { "column": 13, "row": 13 },
        "location"     : { "column": 12, "row": 13 }
      }
    ]
  },
  "location"     : { "column": 12, "row": 12 },
  "message"      : "align consecutive `=` operators"
}
```

An editor applies the edits in array order, and *Prose* guarantees the spans do not overlap, so within one diagnostic the order does not matter.

The [**Editor**](/integrations/editor#structured-diagnostics) integration page covers reading this format from an editor.

### Summary Envelope

A final record closes every `json` run, carrying run-wide counts so a reader gets file and rule totals without adding up the per-diagnostic records. It prints even when the run found nothing, with every finding count at zero and the record as the stream's only line.

```json-vue
{
  "kind"                   : "summary",
  "diagnostics_total"      : 12,
  "files_changed"          : 3,
  "files_visited"          : 47,
  "files_with_diagnostics" : 3,
  "lint_total"             : 0,
  "prose_version"          : "{{ $frontmatter.proseVersion }}",
  "rules_fired"            : { "align-equals": 8, "alphabetize-siblings": 4 },
  "schema_version"         : 1
}
```

| Field | Type | Meaning |
|---|---|---|
| `kind` | `"summary"` | Record kind |
| `diagnostics_total` | integer | Findings printed across the run |
| `files_changed` | integer | Files whose formatting would change |
| `files_visited` | integer | Files the run checked or formatted, leaving out any it refused or failed to parse |
| `files_with_diagnostics` | integer | Files carrying at least one finding |
| `lint_total` | integer | Lint findings among `diagnostics_total` |
| `prose_version` | string | The *Prose* release that printed the record |
| `rules_fired` | `{ "<slug>": count }` | Findings per rule, sorted by key so the output is deterministic |
| `schema_version` | integer | Increments on any change to an existing field's shape, whereas an added field arrives unversioned |
| `unstable` | array of `{ file, rules }` | Each file a second run would change, beside the `--select` list its [**unstable-output**](/reference/cli#unstable-output) notice names, present only when the run finds one |

A file whose syntax its `target-version` rejects prints one `"syntax_error"` record ahead of the summary in place of its diagnostics, carrying `filename`, `cell` for a notebook, `location`, `end_location`, and a `message` naming the version.

```json
{
  "kind"         : "syntax_error",
  "end_location" : { "column": 5, "row": 1 },
  "filename"     : "src/module.py",
  "location"     : { "column": 1, "row": 1 },
  "message"      : "Cannot use `lazy` import statement on Python 3.14 (syntax was added in Python 3.15) under `target-version = \"3.14\"`"
}
```

`schema_version` versions the whole NDJSON contract, so a reader can pin its parsing to a known version and still read the fields added since. The summary record, the leading `kind` field, and each edit's `before` were all added this way, so a reader that reads only `code`, `location`, and `message` parses them unchanged.

## `github`

[**Workflow commands**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub Actions renders as annotations beside the line each finding covers on the PR diff, one line per diagnostic:

```
::warning file=src/module.py,line=14,col=8,endLine=14,endColumn=9::align consecutive `=` operators
```

A file whose syntax its `target-version` rejects prints an `::error` command at that position in place of its warnings.

`endLine` and `endColumn` print only when the finding stays on one line. A finding spanning several lines prints only `line` and `col`, because GitHub's annotation UI shows only the start of a multi-line span. The annotation message is the finding in the rule's words, led by `cell <n>:` for a finding inside a notebook cell.

The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow that reads this format.

## `sarif`

One [**SARIF 2.1.0**](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) document describing the whole run as a single `runs[0]` entry. Each `results[]` entry carries the rule slug as `ruleId`, the source position as a `physicalLocation`, and, for a diagnostic carrying a fix, the replacement as a `fixes[]` entry with an `artifactChanges[]` payload. A file whose syntax its `target-version` rejects lands as an `error`-level result under the `ruleId` `syntax-error`. The document is deep enough that an inline sample would be harder to read than the schema linked above.

The document is serialized through [**`serde-sarif`**](https://docs.rs/serde-sarif/) against the upstream JSON schema, so any SARIF reader accepts it. The [**SARIF Upload**](/integrations/github-actions#sarif-upload) section of the GitHub Actions page covers the step that uploads it to the Security tab.
