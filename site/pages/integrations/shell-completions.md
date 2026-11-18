---
summary: 'Tab-completes every flag and rule slug in Bash, Zsh, Fish, Elvish, and PowerShell.'
tagline: 'interactive shell'
---

# Shell Completions

`prose completions <shell>` prints a completion script for the named shell to stdout. The widget below names the install for each shell, where <Tool slug="bash" />, <Tool slug="elvish" />, <Tool slug="fish" />, and <Tool slug="zsh" /> redirect the script into the file that shell reads completions from, whereas <Tool slug="powershell" /> adds a line to its profile that runs the command at each shell start.

<ShellCompletions />

## Completed Values

Every flag, every enum-valued flag *(`--output-format text|json|github|sarif`, `--color always|auto|never`)*, and every rule slug *(every entry in [**Pipeline Order**](/reference/pipeline-order))* appears in the completion menu. `--select` and `--ignore` take comma-separated slugs, and the script completes the slug at the cursor.

The script is generated from the flags and rules compiled into the `prose` binary, so it lists every rule the binary ships. A rule turned off in `[tool.prose]` still completes, because the menu reads the binary's rule list rather than the project's config.

## Updating After an Upgrade

A new release that adds a flag or a rule shows it in the menu once the script is regenerated. On Bash, Elvish, Fish, and Zsh, re-running the install command from the widget against the upgraded binary overwrites the script, and the next shell session reads the new one. The line in PowerShell's profile generates the script at each shell start, so the next session after an upgrade completes from the new binary, and a second run of the install only adds a duplicate line.

The [**CLI Reference**](/reference/cli) lists every flag. Completions install after the binary is on `PATH`, so the [**Installation**](/guide/installation) chapter comes first.
