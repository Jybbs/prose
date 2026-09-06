---
summary: 'Tab-completes every flag and rule slug in Bash, Zsh, Fish, Elvish, and PowerShell.'
tagline: 'interactive shell'
---

# Shell Completions

`prose completions <shell>` prints a completion script for the named shell to stdout. Redirect it into the directory that shell reads completions from, which the widget below names for each shell, so the install is one command on every platform.

<ShellCompletions />

## What Gets Completed

Every flag, every enum-valued flag *(`--output-format text|json|github|sarif`, `--color always|auto|never`)*, and every rule slug *(every entry in [**Pipeline Order**](/reference/pipeline-order))* appears in the completion menu. `--select` and `--ignore` take comma-separated slugs, and the script completes the slug at the cursor.

The script is generated from the flags and rules compiled into the `prose` binary, so it lists every rule the binary ships. A rule turned off in `[tool.prose]` still completes, because the menu reads the binary's rule list rather than the project's config.

## Updating After an Upgrade

A new release that adds a flag or a rule shows it in the menu once the script is regenerated. Re-run the install command from the widget against the upgraded binary to overwrite the script, and the next shell session reads the new one.

The [**CLI Reference**](/reference/cli) lists every flag. Completions install after the binary is on `PATH`, so the [**Installation**](/usage/installation) chapter comes first.
