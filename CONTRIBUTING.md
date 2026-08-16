# Contributing

***Prose*** formats Python to be legible at a glance, and every rule it ships answers to a snapshot fixture holding the exact source it reads and the exact source it produces. That pairing is what makes a contribution here cheap to review, because a change to behavior arrives as a readable before-and-after in Python rather than as a claim about Rust.

This page walks the path a bug report takes, from the notice the formatter prints to the fixture case a fix is reviewed against. The [**rule catalog**](https://prose.fyi/rules/) covers what each rule does, the [**configuration reference**](https://prose.fyi/reference/configuration) covers every key, and `README.md` covers provisioning the toolchain.

---

## 🐞 Reporting an Unstable Rewrite

Running *Prose* twice should leave the second run nothing to do, and that promise holds for whichever subset of rules a project enables rather than for the default set alone. A `prose format` run that rewrites a file therefore re-applies its enabled rules to the output it just wrote. Where any of them still edits, the run says so:

```console
$ prose format src/module.py
🐞 prose rewrote src/module.py to output a second run would change.

The defect lies in prose rather than in the file, so reproduce it now and confirm
it gone after an upgrade with:

    prose format --select align-equals src/module.py

Filing is one click, the form already carrying the version, the reproducing slugs, the
resolved configuration, the source, and both passes:

    https://github.com/Jybbs/prose/issues/new?template=unstable-output.yml&…

--- src/module.py (first pass)
+++ src/module.py (second pass)
@@ -1,3 +1,3 @@
-alpha     = 1
-beta      = 2
-long_name = 3
+alpha      = 1
+beta       = 2
+long_name  = 3

🗞️ Reformatted 1 file.
🐞 1 file would change on a second run.
```

The `--select` list is not every rule that ran, but the smallest subset that still reproduces, narrowed to one rule where one rule suffices and to a rule pair where two only disagree together. That narrowing is what turns a whole-pipeline symptom into a located defect, and it is the same search the repository's own corpus probe runs.

The rewrite still landed, and the run's exit code reads off that rewrite alone. The defect belongs to the formatter rather than to the source beneath it, so refusing to write would punish the file for the tool's fault. A project that would rather gate CI on the promise opts in through `prose check --validate`, which prints the same notice and takes the failing status that flag already carries.

The invocation is the same one that confirms the fix. Running it again after upgrading either reproduces the defect or prints nothing, so nobody has to wait on a release note to find out.

Opening the link lands the form with the version, the reproducing slugs, the resolved `[tool.prose]` table, the source, and both passes already filled. A large file overflows what a URL carries, so a field that arrives blank is one to paste in from the run.

Editors see the same notice. `prose server` sends it once per document per session rather than on every save, and where the client advertises `window/showDocument` it offers the pre-filled form as an action beside the message.

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
| `mise run rust:settle` | Formats a corpus twice and names every file the second run changes |
| `mise run rust:subsets` | Probes each rule alone and each ordered rule pair over a corpus for one-pass settling |

Both corpus tasks default to the interpreter's own standard library and take a directory argument to aim elsewhere. `rust:settle` answers whether a defect reproduces at all and over how many files, whereas `rust:subsets` locates it in the rule that carries it rather than in whichever pipeline happened to surface it. A fix answers to the second.

The fixture tree is itself a corpus, and the same probe sweeps it on every `cargo test`, failing the suite on any subset that needs a second pass. `rust:subsets` is that probe aimed at a wider corpus and built in release, which is why it stays a local tool rather than a per-pull-request check. Adding the reported case to the tree therefore turns the report into a permanent guard in the same stroke.

---

## 🧵 Opening a Pull Request

Branches cut from `main` and merge back through a squash merge, and every pull request links the issue it closes. A change carries its own evidence in the same commit as the code, which means fixture cases for a change in rule behavior, inline tests for a change in a primitive, and the documentation page for a change in a public surface. The [**pipeline order**](https://prose.fyi/reference/pipeline-order) reference explains where a rule sits and what it may assume the rules ahead of it have already settled, which is usually the first question a rule fix has to answer.
