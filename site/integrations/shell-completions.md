# Shell Completions

`prose completions <shell>` prints a shell-completion script to stdout, ready to redirect into the shell's completion search path. **Five** shells are supported, each carrying the canonical install path the shell expects, meaning the install reduces to a single redirect on every supported platform.

## Zsh

```bash
prose completions zsh > "${fpath[1]}/_prose"
```

The `${fpath[1]}` expansion lands at the first entry of zsh's function path, which is where `compinit` picks up new completions. Restart the shell or run `autoload -Uz compinit && compinit` to pick the completions up without re-launching.

## Bash

```bash
prose completions bash > /etc/bash_completion.d/prose
```

The `/etc/bash_completion.d/` directory is the system-wide completion hook on most distributions. For a per-user install, write to `~/.local/share/bash-completion/completions/prose` instead.

## Fish

```bash
prose completions fish > ~/.config/fish/completions/prose.fish
```

Fish picks up completions in `~/.config/fish/completions/` on the next shell start, no `source` required.

## Elvish

```bash
prose completions elvish > ~/.config/elvish/lib/prose-completions.elv
use prose-completions
```

The `use` line goes in `~/.config/elvish/rc.elv` to register the completions on shell start.

## PowerShell

```powershell
prose completions powershell > $PROFILE.CurrentUserAllHosts
```

PowerShell loads `$PROFILE.CurrentUserAllHosts` on every session, so the completions register on the next shell start.

## What Gets Completed

Every flag, value-enum *(`--output-format text|json|github|sarif`, `--color always|auto|never`)*, and rule slug *(every entry in [**Pipeline Order**](/reference/pipeline-order))* surfaces in the completion menu. The `--select` and `--ignore` flags accept comma-separated rule slugs, and the completion script offers the rule list at the cursor position.

For the canonical CLI surface, see the [**CLI Reference**](/reference/cli) page. Completions install after the binary lands on `PATH`, so the [**Installation**](/guide/installation) chapter covers the prerequisite step.
