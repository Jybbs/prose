# Output Formats

`--output-format` selects the diagnostic shape *Prose* emits, with named formats covering the common consumers. `text` is the human-readable default, rendering rustc-style snippets with carets and fix suggestions. `json` emits Ruff-shaped NDJSON for editor plugins and tooling, wherein the record shape mirrors what LSP-style diagnostic surfaces already consume. `github` emits workflow commands that <Tool slug="github" /> renders as inline annotations. `sarif` emits a [**SARIF 2.1.0**](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) run document for upload into [**GitHub Code Scanning**](https://docs.github.com/en/code-security/code-scanning), persisting findings across runs in the repository's Security tab.

The format selection is per-invocation, defaulting to `text`. All formats write diagnostics to stdout, and operational errors *(parse failures, IO errors, config errors)* go to stderr with an `error:` prefix, so a CI pipeline can split the two streams without inspecting exit codes.

::: warning Diff Mode Is Text-Only
`--diff` requires `text` *(the diff is the text-format presentation)*. Any other pairing surfaces exit code `4` at parse time.
:::

## `text`

The default format, rendering each diagnostic as a rustc-style snippet with a primary annotation marking the offending range, a label naming the rule, and *(when the rule auto-fixes)* a HELP block showing the replacement.

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

The renderer uses [**`annotate-snippets`**](https://docs.rs/annotate-snippets/) for the snippet shape and [**`anstream`**](https://docs.rs/anstream/) for color handling, so the `--color` global flag controls ANSI sequences cleanly.

## `json`

NDJSON shape, one Ruff-compatible record per diagnostic on its own line, closed by a summary envelope. The shape mirrors what Ruff and ESLint publish, so editors with LSP-style diagnostic surfaces map the records onto inline squiggles and the `fix` payload drives quick-fix actions. Every record opens with a `kind` discriminator, wherein per-diagnostic records carry `kind: "diagnostic"` and the closing envelope carries `kind: "summary"`, so a streaming consumer dispatches on the first parsed property without inferring from field presence.

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
| `kind` | `"diagnostic"` | Record discriminator, always the first key |
| `code` | string | The [[rule-id]] slug |
| `filename` | string | Source path |
| `location` | `{ row, column }` | One-indexed start position |
| `end_location` | `{ row, column }` | One-indexed end position |
| `message` | string | The rule's imperative |
| `fix` | object \| null | `null` for lint-only diagnostics, otherwise `{ applicability, edits }` |
| `fix.applicability` | `"safe"` \| `"unsafe"` \| `"display"` | Confidence the edits preserve runtime semantics |
| `fix.edits` | array of `{ before, content, location, end_location }` | Replacement spans the editor or CI can apply |
| `fix.edits[].before` | string | The original substring at the edit's range, paired with `content` for a before/after view without re-reading source |

`applicability` is `"safe"` for every auto-fix *Prose* emits at the current release, matching the Ruff-shared scale wherein `safe` means the rewrite preserves runtime semantics and editors can apply the fix automatically. The `unsafe` and `display` levels exist in the schema for forward compatibility with rules whose rewrites might change observable behavior, with no shipped *Prose* rule emitting at those levels today.

An auto-fix that touches several lines emits one diagnostic with one entry per line in `fix.edits`. The `align-equals` example padding three consecutive assignments surfaces as:

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

Editors apply the edits in array order, and *Prose* guarantees the spans don't overlap, leaving the application order-independent within one diagnostic.

The [**Editor**](/integrations/editor) integration page covers wiring this format into VSCode, Neovim, and the other editors that consume Ruff-shaped diagnostics.

### Summary Envelope

A final record closes every `json` run, carrying run-wide rollup so a consumer reads file and rule counts without re-aggregating the per-diagnostic stream. It emits even when the run surfaces zero diagnostics, where every count lands at zero and the envelope is the stream's only line.

```json
{
  "kind"              : "summary",
  "diagnostics_total" : 12,
  "files_changed"     : 3,
  "files_visited"     : 47,
  "prose_version"     : "0.3.0",
  "rules_fired"       : { "align-equals": 8, "alphabetize": 4 },
  "schema_version"    : 1
}
```

| Field | Type | Meaning |
|---|---|---|
| `kind` | `"summary"` | Record discriminator |
| `diagnostics_total` | integer | Diagnostics emitted across the run |
| `files_changed` | integer | Files whose formatting would change |
| `files_visited` | integer | Files the run processed |
| `prose_version` | string | The emitting *Prose* release |
| `rules_fired` | `{ "<slug>": count }` | Per-rule diagnostic counts, key-sorted for deterministic output |
| `schema_version` | integer | Bumps on any breaking change to existing field shapes, leaving additive fields to land unversioned |

The `schema_version` field versions the whole NDJSON contract. It bumps when a release reshapes an existing field and stays put when a release only adds fields, so a consumer pins parsing against a known version and still reads newer additive fields. The envelope itself, the leading `kind` discriminator, and each edit's `before` arrived this way, additive over the bare per-diagnostic record, leaving a consumer that reads only `code`, `location`, and `message` parsing unchanged.

## `github`

[**Workflow commands**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub Actions renders as native check-run annotations next to the offending line on the PR diff. One line per diagnostic:

```
::warning file=src/module.py,line=14,col=5,endLine=14,endColumn=8::align consecutive `=` operators
```

The `endLine` and `endColumn` fields surface only when the diagnostic stays on one line. Multi-line diagnostics emit only `line` and `col`, because GitHub's annotation UI surfaces only the start position for cross-line spans. The rule's imperative carries through as the annotation message, with the rule slug accessible through the annotation's *"Show context"* expansion.

The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow shape that consumes this format.

## `sarif`

A single [**SARIF 2.1.0**](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) document representing the whole invocation as one `runs[0]` entry. Per-diagnostic `results[]` entries carry the rule slug as `ruleId`, the source position as a `physicalLocation`, and *(when the rule auto-fixes)* the replacement as a `fixes[]` entry with an `artifactChanges[]` payload. The full document is deep enough that an inline sample fights legibility more than it helps, with the canonical schema linked above.

Upload the SARIF file through GitHub's CodeQL action to surface findings in the repository's Security tab:

```yaml
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

The SARIF persists across runs and tracks finding history per rule, so a project sees diagnostic counts trending over time without further wiring. The serialization uses [**`serde-sarif`**](https://docs.rs/serde-sarif/) against the upstream JSON schema, so the document validates against any SARIF consumer.

## Composition with `--diff`

`--diff` is mutually exclusive with `json`, `github`, and `sarif`. The diff is itself the text-format presentation of `format`, so the combination has no defined semantics and the CLI rejects it at parse time with exit code 4.

For the per-subcommand flag list and exit-code semantics, see the [**CLI Reference**](/reference/cli) and the [**Exit Codes**](/reference/exit-codes) reference.
