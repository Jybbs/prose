# Contributing

***Prose*** formats Python to be legible at a glance, and every rule it ships answers to a snapshot fixture holding the exact source it reads and the exact source it produces. That pairing is what makes a contribution here cheap to review, because a change to behavior arrives as a readable before-and-after in Python rather than as a claim about Rust.

This page walks the path a bug report takes, from the notice the formatter prints to the fixture case a fix is reviewed against. The [**rule catalog**](https://prose.fyi/rules/) covers what each rule does, the [**configuration reference**](https://prose.fyi/reference/configuration) covers every key, and `README.md` covers provisioning the toolchain.

---

## 🐞 Reporting an Unstable Rewrite

Running *Prose* twice should leave the second run nothing to do, and that promise holds for whichever subset of rules a project enables rather than for the default set alone. A `prose format` run that rewrites a file therefore re-applies its enabled rules to the output it just wrote. Where any of them still edits, the run says so:

```console
$ prose format src/module.py
🐞 prose rewrote src/module.py to output a second run would change.
...
    prose format --select align-equals src/module.py
```

The [**CLI reference**](https://prose.fyi/reference/cli#unstable-output) carries the whole notice, including the pre-filled form link, the diff between the two passes, and how a run over a tree folds several files into one block.

The `--select` list is not every rule that ran, but the smallest subset that still reproduces, narrowed to one rule where one rule suffices and to a rule pair where two only disagree together. That narrowing is what turns a whole-pipeline symptom into a located defect, and it is the same search the repository's own corpus probe runs.

The rewrite still landed, and the run's exit code reads off that rewrite alone. The defect belongs to the formatter rather than to the source beneath it, so refusing to write would punish the file for the tool's fault. A project that would rather gate CI on the promise opts in through `prose check --validate`, which prints the same notice and takes the failing status that flag already carries.

The invocation is the same one that confirms the fix. Running it again after upgrading either reproduces the defect or prints nothing, so nobody has to wait on a release note to find out. Each run rewrites the file again, so capturing the two passes the form expects goes through the `--stdin` shape the form itself shows rather than through the in-place invocation.

Opening the link lands the form with the version, the reproducing slugs, the resolved `[tool.prose]` table, the source, and both passes already filled. A large file overflows what a URL carries, so a field that arrives blank is one to paste in from the run.

Editors see the same notice, with `prose server` sending it once per document per session rather than on every save, and offering the pre-filled form as an action beside the message where the client advertises `window/showDocument`.

Turning the notice off entirely is one key:

```toml
report-unstable-output = false
```

---

## 🗞️ Turning a Report Into a Fixture Case

A report becomes a fix fastest when it arrives as a case directory. The harness discovers cases by walking the tree, so adding one is creating a directory and nothing else:

```
crate/tests/fixtures/<domain>/<case>/
├── config.toml      optional, a [tool.prose] table for this case alone
├── input.py         the Python the rule reads
└── meta.toml        the title and description the docs site renders
```

### Picking the Domain

The domain is the parent directory, and it decides which rules run over the input. A rule's own slug in snake_case (*`align_equals`, `reflow_collections`, `wrap_docstrings`*) runs that rule alone, which is what an unstable-rewrite report calls for, since the snapshot then shows one rule's effect with no second rule's edits mixed in. The `composition` and `thematic` buckets run the full default pipeline instead, for a case whose point is how several rules compose.

Where the report named an ordered pair rather than a single rule, the pair's shared behavior belongs in `composition`, with the case description naming both slugs.

### Naming the Case

A case name stands on its own and is unique across the whole fixture tree, so a failing test names its scenario without leaning on the directory above it. Write what the case pins:

- `second_pass_widens_the_column` says what happens
- `basic`, `nested`, and `idempotent` say nothing, and a tree-wide `idempotent` tells a reader nothing about which rule broke
- `align_equals_column` repeats the domain the parent directory already carries

Uniqueness is machine-checked, so a duplicate name fails the suite naming both domains.

### Writing the Input

`input.py` opens directly at the code the rule acts on and carries no narration docstring, because what the case pins lives in the `meta.toml` description where it typesets as Markdown:

```python
alpha     = 1
beta      = 2
long_name = 3
```

One trap is worth knowing, in that a case under a rule-slug domain runs that rule alone, so an input pre-carrying a column some *other* rule would have set renders a broken column in the snapshot, reading as though the rule under test did the damage. Keep a column whose driver the rule leaves alone, and drop one whose driver it rewrites.

The exception to the no-docstring rule is a docstring that is itself the subject, which is every case under `wrap_docstrings`, `frame_docstrings`, and `expand_docstrings`, plus any case whose docstring is the structural element under test.

### Writing the Metadata

Every case carries a `meta.toml` with a `[docs]` table:

```toml
[docs]
previewable = true
title       = "A Second Pass Widens the Aligned Column"

description = '''
The first run aligns the `=` column across the three bindings and a second run
widens it again, because the rule measures the row it read rather than the row
it writes.
'''
```

| Key | Meaning |
|---|---|
| `title` | Title-case sentence naming what the case pins |
| `description` | Markdown prose the rule's docs page renders beneath the example |
| `previewable` | Whether the case joins the More Examples list on its rule's page |
| `canonical` | Marks the one lead example per rule page, implying `previewable` |
| `sandbox` | Opts the case into the interactive sandbox's seed pool |

A coverage-only case sets `previewable = false` and still carries a title and description, so it stays self-documented in the tree without crowding the rule's page.

### Running and Reviewing

The suite generates each snapshot beside its input, and reviewing them is what accepts the output as correct:

```bash
mise run rust:test
mise review
```

`mise review` opens every pending snapshot as a diff. Read the Python it proposes as a reader would rather than only checking that the branch is covered, and accept only what reads the way the rule should leave it, since an accepted snapshot becomes the living specification of what that rule produces.

---

## 🪄 Working the Repository

*Prose* is a Rust crate shipping as a Python wheel, with [**mise**](https://mise.jdx.dev) managing the Rust toolchain, the Python interpreter, and every supporting tool from one config. `README.md` carries the provisioning steps, and `mise tasks` lists the full set. These matter most while chasing an unstable rewrite:

| Task | What it does |
|---|---|
| `mise ci` | The full local sweep every pull request answers to |
| `mise run rust:test` | The Rust suites, including every fixture snapshot |
| `mise run rust:settle` | Sweeps a corpus at every line length for rewrites a second pass would change and fixes the output never took |
| `mise run rust:mutations` | Formats every mutation of a corpus with the unstable-output notice on, generating the set under a budget into the directory `PROSE_SETTLE_MUTATIONS` names, or a scratch one, when that directory is absent |
| `mise run rust:subsets` | Probes each rule alone and each ordered rule pair over a corpus for one-pass settling |
| `mise run rust:imports` | Imports each module of a corpus before and after formatting and reports the modules the rewrite breaks, each attributed to the frame it raises in and the rules whose fixes reach it |
| `mise run rust:delta` | Formats a corpus with this tree at every line length and reports what differs from a baseline worktree's formatting, or from a stage `rust:bake` wrote, rule by rule and file by file |
| `mise run rust:bake` | Formats a corpus with one tree at every line length into a tagged stage and writes its mutation set, so a later `rust:delta` or `rust:mutations` reads the baseline rather than rebuilding it |

Every corpus task defaults to the interpreter's own standard library and takes a directory argument to aim elsewhere, `rust:delta` taking it after the baseline worktree it compares against. `rust:imports` is the exception, taking a single module instead, narrowing the run rather than moving the corpus, because it executes what it formats and so answers to the interpreter that owns the tree. `rust:settle` answers whether a defect reproduces at all and over how many files, whereas `rust:subsets` locates it in the rule that carries it rather than in whichever pipeline happened to surface it, and `rust:delta` shows what a fix changed across the corpus once it lands. A fix answers to the second. Every corpus task builds on the `probe` profile, which keeps `debug_assert!` and unwinding while skipping the link-time optimization the release profile pays for, and `rust:probe` builds that profile's binary, sweep generators, and harnesses ahead of a sweep. `PROSE_SETTLE_WIDTHS` names the line lengths a sweep covers as a space-separated list, `PROSE_SETTLE_AXES` narrows the settle sweep to any of the `code`, `docstring`, `import`, and `fallback` budget axes, and `PROSE_DELTA_WIDTHS` does the same for the delta's width set.

`rust:mutations` mutates the corpus `rust:settle` sweeps and formats each variant with the unstable-output notice on, so a rewrite that settles over the corpus as written but not over a parseable edit of it still surfaces. Each mutation edits the module's concrete syntax tree, reordering top-level statements and class-body members, widening and narrowing identifiers, wrapping a sample of calls' arguments in redundant parentheses, injecting comments and suppression directives, and converting the line endings to CRLF, with every variant compiled before it lands so the report names a defect in *Prose* rather than one the mutation introduced. A second positional argument moves the pass off its sixty-second budget, and `PROSE_SETTLE_MUTATIONS` names the directory the set lives in, one `rust:bake` wrote or one the pass writes itself when the directory is absent. The pass formats that set in place, so a directory reused across runs carries the previous run's output rather than the mutated input, and the regeneration line the task prints rebuilds the input. A module the tree parser does not model, a template string among them, yields no variant, so read the denominator on every count before comparing two runs.

`rust:imports` answers the question the settle probes cannot, whether a rewrite that settles still imports, by copying the corpus, formatting the copy, and executing every module the formatter rewrote from both trees in a fresh interpreter with its own tree first on `sys.path`, so a relative import resolves and a rewrite of one module reaches every module importing it. It runs as a `cargo test` target beside the settle and subset sweeps, formatting through the library rather than the binary, and the only Python it carries is the probe that loads one module inside the interpreter under test and reports the names it bound. A module counts as broken where the original imports cleanly and the formatted copy raises, times out, or binds a different namespace, with a second run of both sides confirming it, and the report keys each break by the frame it raises in, why it broke, and the rules whose fixes cover that line or dropped the binding of the name it misses, carrying one line per distinct defect with the modules it reached counted, the diff around the row it names, and the one-module command that reproduces it, leaving out every break the baseline already carries. `PROSE_IMPORTS_PYTHON` names the interpreter whose standard library the sweep runs, `PROSE_SETTLE_WIDTHS` adds widths beside the default, `PROSE_IMPORTS_TIMEOUT` bounds one module's run, and `PROSE_IMPORTS_BAKE` writes the break set a later run under `PROSE_IMPORTS_BASELINE` ratchets against, failing only on a break the baseline does not carry.

The fixture tree is itself a corpus, and the same probe sweeps it on every `cargo test`, failing the suite on any subset that needs a second pass at any swept `code-line-length`, because a subset that settles at one budget can still edit its own output at another. `rust:subsets` is that probe aimed at a wider corpus, narrowed to the shipped default budget so the pointed sweep's wall clock holds, with `PROSE_SETTLE_RULES` keeping only the subsets that touch the rules it names, `PROSE_SETTLE_PAIRS` choosing whether a scoped run claims every pair touching its rules or only the pairs its own rules open, `PROSE_SETTLE_SHARD` taking one `k/n` share of the pairs, and `PROSE_SETTLE_VERIFY` folding every pair beside its chained run and failing on a divergence. A set of runs whose scopes partition the catalog takes the second claim, so each pair is probed once across the set rather than once per endpoint. Adding the reported case to the tree therefore turns the report into a permanent guard in the same stroke.

The `🪻 Corpus` workflow runs the sweeps against the pinned interpreter's standard library on every pull request touching the crate, the mutation pass as its own `🎨 Mutations` job beside `⚓ Settle`, restoring the stage `rust:bake` writes on each `main` push and the mutation set it writes once per revision of the generator, gating on the settle, mutation, import, and subset verdicts, rendering the delta into its job's log as a report, and running the subset probe as one row per rule family named by the family's badge, each scoped to the rules the pull request touches under `crate/src/rules/` where nothing else the workflow watches changed and carrying the whole family otherwise. The `🪟 Imports` job reads the break set that same `main` push bakes beside the stage and fails only on a break the set does not carry, so a module `main` already breaks does not fail a pull request that leaves it alone.

---

## 🧵 Opening a Pull Request

Branches cut from `main` and merge back through a squash merge, and every pull request links the issue it closes. A change carries its own evidence in the same commit as the code, which means fixture cases for a change in rule behavior, inline tests for a change in a primitive, and the documentation page for a change in a public surface. The [**pipeline order**](https://prose.fyi/reference/pipeline-order) reference explains where a rule sits and what it may assume the rules ahead of it have already settled, which is usually the first question a rule fix has to answer.
