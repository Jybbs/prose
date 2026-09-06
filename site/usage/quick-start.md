# Quick Start

`prose format` rewrites files in place, `prose check` reports what would change without writing anything, and `prose completions` prints a shell-completion script. `format` and `check` use the same exit codes, so a CI step and a local pre-commit hook read the same outcomes.

## Run on a Project

`prose format path/to/project` reads every Python file under the directory, runs every enabled rule, and writes the result back:

```bash
prose format path/to/project
```

Three variants cover the other common cases:

1. `prose check` is the CI command. It reads the same files and runs the same rules, prints each pending change as a diagnostic, sets the exit code, and writes nothing.
2. `prose format --diff` is the preview. It prints a unified diff to stdout instead of writing the files.
3. `prose check --stdin` *(also `prose format --stdin`, or a `-` argument on either subcommand)* reads one file's contents from stdin and prints the diagnostics or the rewritten file to stdout, which is the form an editor's save hook uses.

```bash
prose check path/to/project
prose format --diff path/to/project
prose check --stdin < file.py
prose format - < file.py
```

## Which Files Get Walked

Given a directory, *Prose* reads every Python file under it that `.gitignore`, `.ignore`, and your global Git ignore file do not exclude. Vendored dependencies, build output, and anything else a `.gitignore` covers stay out of the run, so `prose format .` at a project root reaches the same files `git ls-files` lists, minus any binary ones.

To skip a path without listing it in `.gitignore` *(a generated directory the project commits, a migrations folder, a third-party snapshot)*, add it to an `.ignore` file at the project root or in any directory above the path. *Prose* has no `.proseignore` file and no `exclude` config key, because the `.ignore` convention `ripgrep` and `fd` established already covers the case and the walker reads it with no further setup.

Hidden files and directories *(names starting with `.`)* are skipped, the way `ripgrep` skips them. The walk stays inside the paths you pass, so `prose format src/` reads only `src/` whatever else the project holds. The [[walker]] primitive page covers how the walk works and how the CLI runs it over several roots.

## Subset the Active Rules

`--select` and `--ignore` restrict a run to some of the configured rules. `--select` replaces the configured set outright, so `--select align-equals` runs only `align-equals` whatever `[tool.prose]` turns on or off. `--ignore` removes rules from whichever set would otherwise run, so `--ignore strip-trailing-commas` drops that one rule and keeps the rest. When both flags appear, the selection applies first and the ignored rules are removed from it.

```bash
prose check --select align-equals path/
prose check --ignore strip-trailing-commas path/
prose check --select align-equals,align-colons path/
```

The [**Rules**](/rules/) page lists every rule, with one page per rule showing its canonical example and the cases around it. The [**CLI Reference**](/reference/cli) covers every flag, how the flags combine, and the exit codes.

## Parallelism

A run over a directory formats files in parallel through [**`rayon`**](https://docs.rs/rayon/), one rule pipeline per worker thread, so a large repository finishes in about the time its slowest file takes rather than the sum of every file. Setting `RAYON_NUM_THREADS=1` runs one file at a time, which helps when diagnostics from several files interleave and make one rule's output hard to read. Stdin mode always runs on one thread, since it reads one file.

## Cache

`prose check` and `prose format` use a per-user [**cache**](/reference/cache) by default, so a file that has not changed since the last run is not formatted again. `--no-cache` bypasses it for one run, the `[cache]` table sets its size cap, and `prose cache clean` empties it.

## Where to Go Next

- The [**Ruff**](/integrations/ruff) integration page covers running Ruff in the same project.
- The [**Suppression**](/usage/suppression) chapter covers exempting a line or a block from a rule.
- The [**Exit Codes**](/reference/exit-codes) reference is the contract a CI gate reads.
