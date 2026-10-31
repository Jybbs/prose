---
description: "Walks each command end to end against a sample project."
---

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
3. `prose check --stdin` *(also `prose format --stdin`, or a `-` argument on either subcommand)* reads one file's contents from stdin and prints the diagnostics or the rewritten file to stdout, which is the form an editor uses for an unsaved buffer.

```bash
prose check path/to/project
prose format --diff path/to/project
prose check --stdin < file.py
prose format - < file.py
```

## Files a Walk Reaches

Given a directory, *Prose* reads every Python file under it that `.ignore` does not exclude, and inside a Git repository every one that `.gitignore` and your global Git ignore file do not exclude either. Vendored dependencies, build output, and anything else a `.gitignore` covers stay out of the run, so `prose format .` at a project root reaches close to the Python files and notebooks `git ls-files` lists.

To skip a path without listing it in `.gitignore` *(a generated directory the project commits, a migrations folder, a third-party snapshot)*, add it to an `.ignore` file at the project root or in any directory above the path.

Hidden files and directories *(names starting with `.`)* are skipped, the way `ripgrep` skips them. The walk stays inside the paths you pass, so `prose format src/` reads only `src/` whatever else the project holds. The [[walker]] primitive page covers how the walk works and how the CLI runs it over several roots.

## Subset the Active Rules

`--select` runs only the rules it names, even one `[tool.prose]` turns off, and `--ignore` drops the rules it names from whichever set would otherwise run. The [**Precedence**](/reference/cli#precedence) section of the CLI reference sets out how the two combine.

```bash
prose check --select align-equals path/
prose check --ignore strip-trailing-commas path/
prose check --select align-equals,align-colons path/
```

The [**Rules**](/rules/) page lists every rule, with one page per rule showing its canonical example and the cases around it. The [**CLI Reference**](/reference/cli) covers every flag.

## Parallelism

A run over a directory formats files in parallel through [**`rayon`**](https://docs.rs/rayon/), its worker threads sharing one rule pipeline per configuration the files resolve to. A run prints each file's diagnostics in the order the walk yields the files, whatever the thread count, and `RAYON_NUM_THREADS=1` runs one file at a time, the setting the [[walker]] page gives for debugging a rule. Stdin mode always runs on one thread, since it reads one file.

## Cache

`prose check` and `prose format` use a per-user [**cache**](/reference/cache) by default, so a file that has not changed since the last run is not formatted again.

## Further Reading

- The [**Ruff**](/integrations/ruff) integration page covers running Ruff in the same project.
- The [**Suppression**](/guide/suppression) chapter covers exempting a line or a block from a rule.
- The [**Exit Codes**](/reference/exit-codes) reference is the contract a CI gate reads.
