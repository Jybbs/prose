# Output Formats

`--output-format` selects the format diagnostics print in, with one format per kind of reader. `text` is the default for a person reading a terminal, printing rustc-style snippets with carets and a suggested fix. `json` prints Ruff-shaped NDJSON for editor plugins and tooling, in the record shape editor diagnostic panels already read. `github` prints workflow commands that <Tool slug="github" /> renders as inline annotations. `sarif` prints a [**SARIF 2.1.0**](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) document for upload to [**GitHub Code Scanning**](https://docs.github.com/en/code-security/code-scanning), which keeps findings across runs in the repository's Security tab.

The format is chosen per run and defaults to `text`. Every format writes diagnostics to stdout, and operational errors *(parse failures, IO errors, config errors)* go to stderr with an `error:` prefix, so a CI pipeline can separate the two streams without reading exit codes.

::: warning Diff Mode Is Text-Only
`--diff` needs `text`, since the diff is the text format's own presentation. Any other pairing exits `4` while the arguments are parsed.
:::

## `text`

The default format prints each diagnostic as a rustc-style snippet, where a caret marks the range the finding covers, a label names the rule, and, for a rule that auto-fixes, a HELP block shows the replacement.

```
warning: align consecutive `=` operators
  --> src/module.py:14:5
   |
14 |     foo = 1
   |     ^^^ align-equals
   |
help: replace with
  --> src/module.py:14:5
   |
14 |     foo   = 1
```

A notebook diagnostic prints under a `cell N` header against that cell's own source, with the caret positioned within the cell.

The snippets are rendered by [**`annotate-snippets`**](https://docs.rs/annotate-snippets/) and the color by [**`anstream`**](https://docs.rs/anstream/), so the global `--color` flag controls the ANSI sequences.

## `json`

One Ruff-compatible record per diagnostic on its own line, in NDJSON, closed by a summary record. The record shape matches what Ruff and ESLint print, so an editor with a diagnostics panel shows each record as an inline squiggle and uses the `fix` payload for a quick-fix action. Every record opens with a `kind` field, `"diagnostic"` for a finding and `"summary"` for the closing record, so a streaming reader can dispatch on the first parsed property without inferring the kind from which fields are present.

```json
{
  "kind"         : "diagnostic",
  "code"         : "align-equals",
  "filename"     : "src/module.py",
  "location"     : { "row": 14, "column": 5 },
  "end_location" : { "row": 14, "column": 8 },
  "message"      : "align consecutive `=` operators",
  "fix"          : {
    "applicability" : "safe",
    "edits"         : [
      {
        "before"       : "foo = 1",
        "content"      : "foo   = 1",
        "location"     : { "row": 14, "column": 5 },
        "end_location" : { "row": 14, "column": 12 }
      }
    ]
  }
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
| `message` | string | The rule's imperative |
| `fix` | object \| null | `null` for a lint finding, otherwise `{ applicability, edits }` |
| `fix.applicability` | `"safe"` \| `"unsafe"` \| `"display"` | Confidence that the edits preserve runtime semantics |
| `fix.edits` | array of `{ before, content, location, end_location }` | The replacement spans an editor or CI can apply |
| `fix.edits[].before` | string | The original text at the edit's range, paired with `content` for a before and after view without reading the source again |

For a notebook input, each record adds a `cell` field with the one-indexed cell, and its `location` and `end_location` are positions within that cell rather than offsets into the joined source. A module input has no `cell`, and its positions are absolute.

`applicability` is `"safe"` for every auto-fix *Prose* prints in the current release, on the scale Ruff uses, where `safe` means the rewrite preserves runtime semantics and an editor can apply it without asking. The `unsafe` and `display` levels are reserved in the schema for a future rule whose rewrite might change observable behavior, and no shipped rule prints them today.

An auto-fix that touches several lines prints one diagnostic with one entry per line in `fix.edits`. The `align-equals` example padding three consecutive assignments prints as:

```json
{
  "kind"         : "diagnostic",
  "code"         : "align-equals",
  "filename"     : "src/configure.py",
  "location"     : { "row": 12, "column": 5 },
  "end_location" : { "row": 14, "column": 24 },
  "message"      : "align consecutive `=` operators",
  "fix"          : {
    "applicability" : "safe",
    "edits"         : [
      {
        "before"       : "    timeout = 30",
        "content"      : "    timeout      = 30",
        "location"     : { "row": 12, "column": 1 },
        "end_location" : { "row": 12, "column": 16 }
      },
      {
        "before"       : "    retries = 5",
        "content"      : "    retries      = 5",
        "location"     : { "row": 13, "column": 1 },
        "end_location" : { "row": 13, "column": 16 }
      },
      {
        "before"       : "    backoff_base = 1.5",
        "content"      : "    backoff_base = 1.5",
        "location"     : { "row": 14, "column": 1 },
        "end_location" : { "row": 14, "column": 24 }
      }
    ]
  }
}
```

An editor applies the edits in array order, and *Prose* guarantees the spans do not overlap, so within one diagnostic the order does not matter.

The [**Editor**](/integrations/editor) integration page covers reading this format from VSCode, Neovim, and the other editors that accept Ruff-shaped diagnostics.

### Summary Envelope

A final record closes every `json` run, carrying run-wide counts so a reader gets file and rule totals without adding up the per-diagnostic records. It prints even when the run found nothing, with every count at zero and the record as the stream's only line.

```json-vue
{
  "kind"              : "summary",
  "diagnostics_total" : 12,
  "files_changed"     : 3,
  "files_visited"     : 47,
  "prose_version"     : "{{ $frontmatter.proseVersion }}",
  "rules_fired"       : { "align-equals": 8, "alphabetize-siblings": 4 },
  "schema_version"    : 1
}
```

| Field | Type | Meaning |
|---|---|---|
| `kind` | `"summary"` | Record kind |
| `diagnostics_total` | integer | Findings printed across the run |
| `files_changed` | integer | Files whose formatting would change |
| `files_visited` | integer | Files the run read |
| `prose_version` | string | The *Prose* release that printed the record |
| `rules_fired` | `{ "<slug>": count }` | Findings per rule, sorted by key so the output is deterministic |
| `schema_version` | integer | Increments on any change to an existing field's shape, whereas an added field arrives unversioned |

`schema_version` versions the whole NDJSON contract, incrementing when a release changes the shape of an existing field and staying put when a release only adds fields, so a reader can pin its parsing to a known version and still read newer added fields. The summary record, the leading `kind` field, and each edit's `before` were all added this way, so a reader that reads only `code`, `location`, and `message` parses them unchanged.

## `github`

[**Workflow commands**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub Actions renders as annotations beside the line each finding covers on the PR diff, one line per diagnostic:

```
::warning file=src/module.py,line=14,col=5,endLine=14,endColumn=8::align consecutive `=` operators
```

`endLine` and `endColumn` print only when the finding stays on one line. A finding spanning several lines prints only `line` and `col`, because GitHub's annotation UI shows only the start of a multi-line span. The rule's imperative is the annotation message, and the rule slug appears under the annotation's *"Show context"* expansion.

The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow that reads this format.

## `sarif`

One [**SARIF 2.1.0**](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) document describing the whole run as a single `runs[0]` entry. Each `results[]` entry carries the rule slug as `ruleId`, the source position as a `physicalLocation`, and, for a rule that auto-fixes, the replacement as a `fixes[]` entry with an `artifactChanges[]` payload. The document is deep enough that an inline sample would be harder to read than the schema linked above.

Upload the SARIF file through GitHub's CodeQL action to show findings in the repository's Security tab:

```yaml
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

The Security tab keeps the findings across runs and tracks their history per rule, so a project sees its counts over time with no further setup. The document is serialized through [**`serde-sarif`**](https://docs.rs/serde-sarif/) against the upstream JSON schema, so any SARIF reader accepts it.

## Composition With `--diff`

`--diff` cannot be combined with `json`, `github`, or `sarif`. The diff is the text format's own presentation of a `format` run, so the combination has no meaning and the CLI rejects it while parsing the arguments with exit code 4.

The [**CLI Reference**](/reference/cli) lists each subcommand's flags, and the [**Exit Codes**](/reference/exit-codes) reference covers the codes.
