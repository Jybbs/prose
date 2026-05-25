# Shell Completions

`prose completions <shell>` prints a shell-completion script to stdout, ready to redirect into the shell's completion search path. Each supported shell carries the canonical install path it expects, meaning the install reduces to a single redirect on every supported platform.

<ShellCompletions />

## What Gets Completed

Every flag, value-enum *(`--output-format text|json|github|sarif`, `--color always|auto|never`)*, and rule slug *(every entry in [**Pipeline Order**](/reference/pipeline-order))* surfaces in the completion menu. The `--select` and `--ignore` flags accept comma-separated rule slugs, and the completion script offers the rule list at the cursor position.

The completion script is generated from the `prose` binary's compile-time view of flags and rules, so it carries every rule the binary ships rather than a runtime-filtered subset. A rule disabled in `[tool.prose]` still surfaces in the completion menu, because the menu reads the binary's catalog rather than the project's config.

## Updating After an Upgrade

A new *Prose* release that adds a flag or a rule lands in the completion menu after the script is regenerated. Re-running the install command from the widget above against the upgraded binary writes the new script to the same path, and the next shell session picks up the additions.

For the canonical CLI surface, see the [**CLI Reference**](/reference/cli) page. Completions install after the binary lands on `PATH`, so the [**Installation**](/usage/installation) chapter covers the prerequisite step.
