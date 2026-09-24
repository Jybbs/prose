# Contributing

***Prose*** formats Python to be legible at a glance, and every rule it ships answers to a snapshot fixture holding the exact source it reads and the exact source it produces. That pairing is what makes a contribution here cheap to review, because a change to behavior arrives as a readable before-and-after in Python rather than as a claim about Rust.

This page walks a change through each step in the order a contributor takes it, from setting up a clone and filing or picking up an issue, through the fixture case a fix is reviewed against and the corpus sweeps that test it, to the commit, the pull request, and the checks that run on it. The [**rule catalog**](https://prose.fyi/rules/) covers what each rule does, and the [**configuration reference**](https://prose.fyi/reference/configuration) covers every key.

---

## 🗜️ Setting Up

`README.md` carries the provisioning steps, and once a clone is provisioned, these conventions hold:

| **Convention** | **Rule** |
|---|---|
| Tools | Every tool comes from `.mise/config.toml` at the version it pins, with `.mise/mise.lock` recording each download's checksum for every runner platform. The config's `min_version` names the oldest mise that reads it |
| Tasks | A command runs through its mise task rather than the `cargo`, `bun`, or `maturin` call the task wraps, because the task carries the flags CI runs with. `mise tasks` lists them all, and most developer tasks also run under a one-word alias (*`mise ci` for `repo:ci`, `mise review` for `rust:review`*) |
| Virtualenv | The first mise command in a clone creates `crate/.venv`, and `mise wheel` builds the extension into it through `maturin develop` |
| Lockfiles | An edit to `crate/Cargo.toml`, `crate/pyproject.toml`, `site/package.json`, or `.mise/config.toml` takes a `mise relock` in the same commit, since the `🪵 Lockfile` row (*a row being one of the jobs a workflow runs*) fails a pull request whose lockfile lags its manifest |
| Local sweep | `mise ci` runs what the `🪻 CI` and `🪻 Deploy` workflows run, short of uploading coverage |

---

## 🧵 Filing an Issue

Every issue opens from a template, because the chooser offers no blank issue. It links the rule catalog, the configuration reference, and the sandbox in its place, and one of those usually answers a question about what *Prose* does before an issue is needed.

| **Template** | **For** | **Labels** |
|---|---|---|
| `Unstable output` | *A rewrite a second run would change again, reported through the link the formatter prints, with every field that fits in that link already filled* | `🐞 bug` |
| `Bug` | *Any other defect, such as a wrong rewrite, a crash, or a flag or key that does not do what its reference says, with fields for the version, the command, the resolved configuration, the source, and what happened beside what should have* | `🐞 bug` |
| `Issue spec` | *A unit of work, opening on what is wrong today and what has to change, with numbered requirements only where the work needs three or more steps a reader would not assume* | The labels the author picks |

The formatter fills in the `Unstable output` form from the run itself, and the next section walks through that report in full.

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

## 🗺️ Picking Up an Issue

An issue carries its labels and its milestone before work on it starts, because both are copied onto its pull request, where GitHub reads the labels to file the pull request under a release-notes category.

1. Pick an issue from an open milestone, which is named for the minor release line it ships in (*`0.10`*), and assign it to yourself
2. Confirm the issue carries at least one label from the table below, adding the label for the rule family or area it touches where none is set
3. Cut a branch named `<issue>/<slug>` from an up-to-date `main`, the slug holding at most three terms from the issue's title (*`55/fmt-suppression`*)
4. Commit the work on that branch, and open the pull request once `mise ci` passes

Setting the labels, the assignee, and the milestone takes triage access on the repository, so a contributor without it leaves those three fields to a maintainer.

| **Label** | **Covers** |
|---|---|
| `🐞 bug` | A defect in what the formatter writes or reports, or in a CI workflow |
| `🦉 engine` | Parser integration, the pipeline, `Source`, and the primitives under `crate/src/primitives/` |
| `🪜 alignment` | Rules that pad the space before a shared token so consecutive rows read as columns |
| `🪉 ordering` | Rules that reorder sibling nodes by a fixed key while keeping each node's comments with it |
| `🧺 layout` | Rules that explode a bracketed construct to one entry per line once it outgrows its line |
| `🪶 formatting` | Rules that rewrite a token, a line, or a spelling once a statement's layout is settled |
| `🧶 lint` | Rules that report a finding without rewriting the source |
| `🪄 cli` | Command-line interface, config loader, diff output |
| `📰 docs` | Docstring rules, the README, the contributor guide, rule pages, and in-code doc comments |
| `🗝️ site` | Documentation-site infrastructure, content, and theming |
| `🗺️ architecture` | Cross-cutting structural or design changes |
| `🗜️ build` | `Cargo.toml`, maturin, dependencies, CI workflows, mise tasks, and release plumbing |

`.github/labels.toml` is the registry declaring every label's name, color, and description, so a label is added, renamed, or recolored there rather than in the repository settings. `mise run repo:labels` creates or updates each registry label on GitHub, so a label renamed in the registry is created as a new label beside the old one. The task lists every label on GitHub the registry omits rather than deleting it, leaving a maintainer to rename or delete it by hand. `mise run repo:audit` fails on any of these:

- `.github/release.yml` or an issue template names a label the registry lacks, or the table above names or describes a label differently
- A registry label sits in no release-notes category, or in more than one
- A color is not six lowercase hex digits, or two labels share it
- A rule family's label differs from the color or the description the docs site gives that family

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

| **Key** | **Meaning** |
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

## ⚓ Sweeping a Corpus

A fixture case pins one shape, whereas the corpus tasks run the formatter over a whole body of Python at every line length, which is how a defect surfaces before anyone reports it and how a fix is shown to reach every file it should. The corpus tasks below are the ones that matter most while chasing an unstable rewrite:

| **Task** | **What It Does** |
|---|---|
| `mise run rust:proof` | Formats the fixture tree at every line length, then checks that a second pass changes nothing and that every reported fix was applied, which is the settling half of `rust:test` |
| `mise run rust:settle` | Sweeps a corpus at every line length for rewrites a second pass would change and fixes the output never took |
| `mise run rust:mutations` | Formats every mutation of a corpus with the unstable-output notice on, generating the set under a budget into the directory `PROSE_SETTLE_MUTATIONS` names, or a scratch one, when that directory is absent |
| `mise run rust:subsets` | Probes each rule alone and each ordered rule pair over a corpus for one-pass settling, and holds each rule declaring `PRESERVES_TREE` to its input's tree |
| `mise run rust:imports` | Imports each module of a corpus before and after formatting and reports the modules the rewrite breaks, each attributed to the frame it raises in and the rules whose fixes reach it |
| `mise run rust:delta` | Formats a corpus with this tree at every line length and reports what differs from a baseline worktree's formatting, or from a stage `rust:bake` wrote, rule by rule and file by file |
| `mise run rust:bake` | Formats a corpus with one tree at every line length into a tagged stage and writes its mutation set, so a later `rust:delta` or `rust:mutations` reads the baseline rather than rebuilding it |

Every corpus task defaults to the interpreter's own standard library and takes a directory argument to aim elsewhere, `rust:delta` taking it after the baseline worktree it compares against. `rust:imports` is the exception, taking a single module instead, narrowing the run rather than moving the corpus, because it executes what it formats and so answers to the interpreter that owns the tree. `rust:settle` answers whether a defect reproduces at all and over how many files, whereas `rust:subsets` locates it in the rule that carries it rather than in whichever pipeline happened to surface it, and `rust:delta` shows what a fix changed across the corpus once it lands. A fix answers to the second. Every corpus task builds on the `probe` profile, which keeps `debug_assert!` and unwinding while skipping the link-time optimization the release profile pays for, and `rust:probe` builds that profile's binary, corpus binaries, and harnesses ahead of a sweep. Each of `rust:settle`, `rust:subsets`, `rust:mutations`, `rust:delta`, and `rust:imports` takes one lock shared by every checkout on the machine once its build finishes, so no two of them sweep at once, whereas a `cargo test` run directly takes no lock. A task blocked on another prints the path of the lock it waits on. `PROSE_SETTLE_WIDTHS` names the line lengths a sweep covers as a space-separated list, `PROSE_SETTLE_AXES` narrows the settle sweep to any of the `code`, `docstring`, `import`, and `fallback` budget axes, and `PROSE_DELTA_WIDTHS` does the same for the delta's width set.

`rust:mutations` mutates the corpus `rust:settle` sweeps and formats each variant with the unstable-output notice on, so a rewrite that settles over the corpus as written but not over a parseable edit of it still surfaces. Each mutation edits the module's concrete syntax tree, reordering top-level statements and class-body members, widening and narrowing identifiers, wrapping a sample of calls' arguments in redundant parentheses, injecting comments and suppression directives, and converting the line endings to CRLF, with every variant compiled before it lands so the report names a defect in *Prose* rather than one the mutation introduced. A second positional argument moves the pass off its sixty-second budget, and `PROSE_SETTLE_MUTATIONS` names the directory the set lives in, one `rust:bake` wrote or one the pass writes itself when the directory is absent. The pass formats that set in place, so a directory reused across runs carries the previous run's output rather than the mutated input, and the regeneration line the task prints rebuilds the input. A module the tree parser does not model, a template string among them, yields no variant, so read the denominator on every count before comparing two runs.

`rust:imports` checks what the settle probes cannot, whether a rewrite that settles still imports. It copies the corpus and formats the copy with `target-version` set to the interpreter's own version. It then executes every importable module of the corpus outside its entry points from both trees, each in a fresh interpreter with its own tree first on `sys.path`, so a relative import resolves and a rewrite of one module reaches every module importing it, whether or not the formatter rewrote the importer. It runs as a `cargo test` target beside the settle and subset sweeps, formatting through the library rather than the binary, and the only Python it carries is the probe that loads one module inside the interpreter under test and reports the names it bound. A module counts as broken where the original imports cleanly and the formatted copy raises, times out, or binds a different namespace, with a second run of both sides confirming it. A file the pipeline read but could not format counts as broken too, whereas a file it could not read or parse is counted and left out, as the settle sweep leaves it. Where the second run of the original binds some names differently from the first (*a function address that address-space randomization moves, a timestamp taken on import*), those names are set aside rather than the whole module. The module still counts as broken wherever the formatted copy differs outside them, and the number of names set aside never fails a run on its own.

Where a recorded fix removed the binding of a name, whichever rule wrote that fix, the comparison leaves that name out of the module the fix edited. Every module of the corpus runs, so one that reads the removed name raises as a break of its own, naming that module and the rule. A module outside the corpus that reads a removed name never runs, so the sweep holds no evidence about it. Once a module runs, the probe reads its `__annotations__` the way a consumer reads them, so the comparison covers which names the module annotates at module scope, and a formatted copy whose module annotations raise when read counts as broken. The probe then reads the annotations of each function and class the module defines, and of each function in such a class's body, the way `annotationlib.get_annotations` reads them, so a formatted copy where one of them raises counts as broken, whereas a definition whose annotations raise in the original too is set aside.

The gate requires zero breaks, and it also fails a run on any condition below, each of which leaves it unable to judge whether a module broke:

- A module left no record
- The format pass rewrote no file
- The run compared no module
- A module's original run raised from an import naming the module itself or a package holding it, which means the harness's loader failed rather than the module

The report keys each break by the frame it raises in, why it broke, and the rules whose fixes cover that line or dropped the binding of the name it misses, carrying one line per distinct defect with the modules it reached counted, the diff around the row it names, and the one-module command that reproduces it. It also lists each module whose original run raised or timed out beside how that run ended, and each name the comparison left out beside the fix that removed it. `PROSE_IMPORTS_PYTHON` names the interpreter whose standard library the sweep runs, `PROSE_SETTLE_WIDTHS` adds widths beside the default, and `PROSE_IMPORTS_TIMEOUT` bounds one module's run.

The fixture tree is itself a corpus, and the same probe sweeps it on every `cargo test`, failing the suite on any subset that needs a second pass at any swept `code-line-length`, because a subset that settles at one budget can still edit its own output at another. A pair the registry's independence table declares independent also runs as one pipeline splicing both rules into a single buffer, failing the suite wherever that run differs from the chained one, and a pointed sweep reports how many files each undeclared pair's spliced run agreed on, which is the evidence a table entry rests on. `rust:subsets` is that probe aimed at a wider corpus, narrowed to the shipped default budget so the pointed sweep's wall clock holds, with `PROSE_SETTLE_RULES` keeping only the subsets that touch the rules it names, `PROSE_SETTLE_PAIRS` choosing whether a scoped run claims every pair touching its rules or only the pairs its own rules open, `PROSE_SETTLE_SHARD` taking one `k/n` share of the pairs, and `PROSE_SETTLE_VERIFY` folding every pair beside its chained run and failing on a divergence. A set of runs whose scopes partition the catalog takes the second claim, so each pair is probed once across the set rather than once per endpoint. Adding the reported case to the tree therefore turns the report into a permanent guard in the same stroke.

The subset probe also holds every rule whose type declares the `PRESERVES_TREE` constant as `true` to the tree its input parses to. Each rewrite such a rule makes, pair runs included, has to parse to a tree whose `ComparableModModule` equals its input's, the `ruff_python_ast` form that ignores positions, parentheses, comments, and implicit string concatenation. Every such rule also runs together in one pipeline under the same comparison. An equal tree means the rule changed no node the parser builds, so for these rules the probe reaches every function body in the corpus, including the ones `rust:imports` never executes because nothing calls them while the module loads. A rule that changes the tree by design declares `false` and answers to `rust:imports` alone. Each violation reports the rule, the file, and the width beside a diff of the innermost statement whose tree changed and the command that sweeps that file alone. Over the fixture tree, the suite also fails on a rule declaring `false` whose every rewrite keeps the tree, because a stale `false` would drop that rule from the check without failing anything.

The `🪻 Corpus` workflow runs the sweeps against the pinned interpreter's standard library on every pull request touching the crate or the corpus binaries, the mutation pass as its own `🎨 Mutations` job beside `⚓ Settle`, restoring the stage `rust:bake` writes on each `main` push and the mutation set it writes once per revision of the generator, its manifest, or the lockfile, gating on the settle, mutation, import, and subset verdicts, rendering the delta into its job's log as a report, and running the subset probe as one row per rule family named by the family's badge, each scoped to the rules the pull request touches under `crate/src/rules/` where nothing else the workflow watches changed and carrying the whole family otherwise. The `🪟 Imports` job reads nothing but the corpus and the tree under test, so it fails a pull request on any module the rewrite breaks, whether or not `main` breaks that module too.

---

## ☕ Committing

Every commit on a branch follows the [**Conventional Commits**](https://www.conventionalcommits.org/) shape below. A squash merge folds a branch's commits into one on `main`, whereas the closed pull request keeps each of them as the record of how the change was built.

```
type(scope): concise description

- First thematic change
- Second thematic change
```

| **Part** | **Rule** |
|---|---|
| Type | `feat`, `fix`, `refactor`, `chore`, `docs`, or `test` |
| Scope | `cli`, `config`, `pipeline`, `source`, `aligner`, `orderer`, `rules`, `docs`, `build`, or a rule's slug (*`align-equals`*) |
| Title | Lowercase after the prefix, naming the whole change |
| Body | One hyphen bullet per thematic change, opening on a verb and kept near **fifteen words**, with a backticked token counting as one word |
| Casing | Each bullet's first word capitalized, unless the bullet opens on a backticked identifier |
| One bullet | A body that would hold one bullet folds it into a more descriptive title instead, leaving the commit title-only |
| Left out of the body | Style-only fixes, doc-only updates, and formatting tweaks |
| Attribution | No co-author line and no tool attribution |

Pass the message through a single-quoted heredoc, even for a title-only commit, because the shell runs a backtick inside a double-quoted `-m "..."` string as command substitution, whereas the text a quoted heredoc produces is never expanded:

```bash
git commit -m "$(cat <<'EOF'
chore(build): declare the labels in a registry the audit reads

- Add `.github/labels.toml` with every label's color and description
- Fail `repo:audit` where `.github/release.yml` names a label the registry lacks
EOF
)"
```

A change carries its own evidence in the same commit as the code, which means fixture cases for a change in rule behavior, inline tests for a change in a primitive, and the documentation page for a change in a public surface. The [**pipeline order**](https://prose.fyi/reference/pipeline-order) reference explains where a rule sits and what it may assume the rules ahead of it have already settled, which is usually the first question a rule fix has to answer.

---

## 🪻 Opening a Pull Request

When a pull request is opened in the browser, GitHub pre-fills its body from `.github/PULL_REQUEST_TEMPLATE.md`. The template carries every section below apart from Implementation Notes, and its comments give the title format, the fields to copy from the issue, and the condition for adding Implementation Notes.

| **Field** | **Value** |
|---|---|
| Title | `[N]` followed by the issue's title, where `N` is the number of the issue the pull request closes, keeping every backtick the title places around a rule slug or code token (*`` [576] Rename the `sweeps` workspace member to `corpus` ``*) |
| Labels | The issue's labels |
| Assignee | The issue's assignee |
| Milestone | The issue's milestone |
| Merge | A squash merge, landing one commit on `main` titled with the pull request's title and number, then deleting the branch |

Where a contributor lacks triage access, a maintainer copies the labels, the assignee, and the milestone across instead.

When GitHub generates a release's notes, it files each pull request under the first category in `.github/release.yml` that lists one of the pull request's labels, so a pull request labeled `🐞 bug` beside a family label lands under `🐞 Bugs`, and one carrying no label lands under `Other`.

The body runs through these sections, separated by `---` dividers:

| **Section** | **Holds** |
|---|---|
| `🪻 Quick Summary` | Two or three sentences naming what the pull request delivers |
| `🗞️ Key Changes` | One concrete change per bullet, opening on a third-person verb (*"Adds", "Pins"*) rather than the imperative a commit bullet takes, and naming the function, file, rule slug, or task it touches, with a blank line between bullets |
| `☕ Implementation Notes` | Optional and usually absent, kept only for a point a reviewer needs that no bullet can carry |
| `🧵 Related Issues` | `Closes #N` lines and nothing else |


---

## 🪷 Checks Every Pull Request Runs

A pull request triggers each workflow whose path filter matches a file it touches, a change to the workflow's own file included, and every such workflow ends on a `🗞️ Brief` job that renders the run's summary and passes only when every job it waits on passed:

| **Workflow** | **Fires on a Pull Request Touching** |
|---|---|
| `🪻 CI` | Any file other than Markdown, `LICENSE`, and the docs site, though the docs site's wasm tests still count |
| `🪻 Deploy` | The docs site, the tasks and libraries under `.mise/`, the tool pins and their lockfile, the composite actions, the issue and pull request templates, `.nvmrc`, `CONTRIBUTING.md`, `LICENSE`, `README.md`, or `crate/Cargo.toml` |
| `🪻 Corpus` | The crate's source, the corpus binaries, harnesses, and tasks, the workspace manifests and lockfile, the tool pins, or the composite actions |
| `🪻 Release` | `crate/Cargo.toml`, `crate/pyproject.toml`, the tool pins, the composite actions and step-summary templates, or the tasks and libraries the release rows call |

`🗞️ Brief` is the one check the `main` ruleset requires, so a pull request can merge once every workflow it triggered reports that check green. The rulesets are recorded under `.github/rulesets/`, and `mise run repo:rulesets` applies them along with the repository settings no ruleset covers (*squash as the only merge method, the head branch deleted on merge, every action pinned to a commit, a read-only workflow token*). `mise run repo:audit` fails when the checks `.github/rulesets/main.json` requires differ from the names of the jobs the pull-request workflows end on, so renaming the `🗞️ Brief` job cannot leave every pull request waiting on a check that never reports.

The table lists each row beside the task that runs the same check locally:

| **Row** | **Workflow** | **Runs Locally As** | **What It Checks** |
|---|---|---|---|
| `🪶 Format` | `🪻 CI` | `mise run rust:check` | Rust source matches `rustfmt` |
| `🪵 Lockfile` | `🪻 CI`, `🪻 Deploy` | `mise run lock:check` | Every lockfile matches its manifest |
| `🪷 Audit` | `🪻 CI`, `🪻 Deploy` | `mise run repo:audit` | Each version pin matches every file that repeats it. The label registry matches `.github/release.yml`, the issue templates, the label table above, and each rule family's color and description on the docs site. The checks the `main` ruleset requires match the jobs the pull-request workflows end on. Every tracked file sits inside the path filter of some pull-request workflow, and every file the audit reads sits inside that of a workflow running the audit. No action manifest carries a YAML anchor, the `module.yml` push trigger covers every wasm source path, and `mise tasks validate` passes |
| `🪓 Unused` | `🪻 CI` | `mise run rust:unused` | No `Cargo.toml` declares a dependency its crate never uses |
| `📎 Clippy` | `🪻 CI` | `mise run rust:lint` | `clippy` reports nothing across every target |
| `🗜️ Build` | `🪻 CI` | `mise run rust:build` | The workspace builds in debug |
| `🪚 Suite` | `🪻 CI` | `mise run rust:suite` | Every Rust suite passes apart from the `corpus` and `settle` targets, which `🥃 Proof` runs |
| `🥃 Proof` | `🪻 CI` | `mise run rust:proof` | The fixture tree settles in one pass at every line length, with every reported fix applied |
| `🛶 Wasm` | `🪻 CI` | `mise run wasm:lint` | `clippy` reports nothing in `prose_wasm` for the wasm target |
| `🪃 Browser` | `🪻 CI` | `mise run wasm:smoke` | The packed web module formats Python in a browser smoke test |
| `☂️ Coverage` | `🪻 CI`, `🪻 Deploy` | `mise run rust:coverage`, `mise run site:coverage` | The Rust and docs-site coverage reports reach Codecov, whose status checks fail below **95%** on the project total and on each patch |
| `🪡 Typecheck` | `🪻 Deploy` | `mise run site:typecheck` | `vue-tsc` reports no type error in the docs site |
| `🧶 Lint` | `🪻 Deploy` | `mise run site:lint` | `oxlint` reports nothing in the docs site's TypeScript |
| `🧹 Cruft` | `🪻 Deploy` | `mise run site:cruft` | `knip` finds no dead code or unused dependency in the docs site |
| `🎞️ Press` | `🪻 Deploy` | `mise run site:links` | The docs site builds and every internal link resolves |
| `⚓ Settle` | `🪻 Corpus` | `mise run rust:settle` | The standard library settles in one pass at every line length |
| `🎨 Mutations` | `🪻 Corpus` | `mise run rust:mutations` | Every mutation of the standard library settles in one pass |
| `🦋 Delta` | `🪻 Corpus` | `mise run rust:delta` | The job reports how the branch changes the standard library's formatting against `main`, and never fails |
| `🪟 Imports` | `🪻 Corpus` | `mise run rust:imports` | Every standard-library module that imports before formatting still imports after it, each break attributed to the rules whose fixes reach it |
| `🪜 Subsets · Alignment`, one row per family | `🪻 Corpus` | `mise run rust:subsets` | The family's rules settle in one pass alone and in every ordered pair, and each one declaring `PRESERVES_TREE` keeps the tree its input parses to |
| `🎻` one row per platform, `🚢 sdist`, `🕯️ Validate` | `🪻 Release` | `mise run rust:wheel` | A wheel builds for every platform, the sdist builds, and one wheel installs offline and runs. The local task builds the extension for the current platform alone |

The corpus rows sweep the pinned interpreter's standard library rather than the fixture tree, and [**Sweeping a Corpus**](#-sweeping-a-corpus) above covers their tasks, the variables that narrow them, and how the `🪻 Corpus` workflow scopes its subset rows.
