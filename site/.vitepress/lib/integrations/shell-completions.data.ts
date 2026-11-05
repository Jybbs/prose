import { defineLoader } from 'vitepress'

import type { InlineNode } from '../markdown/inline-nodes'
import * as renderer       from '../markdown/renderer'

interface ShellCompletion {
  caption   : string
  codeHtml  : string
  language  : string
  name      : string
  noteNodes : InlineNode[]
  slug      : string
}

declare const data: readonly ShellCompletion[]
export { data }

type ShellCompletionSource =
  Omit<ShellCompletion, 'codeHtml' | 'noteNodes'> & { code: string, note: string }

const SOURCES: readonly ShellCompletionSource[] = [
  {
    caption  : 'bash_completion.d',
    code     : `prose completions bash > /etc/bash_completion.d/prose`,
    language : 'bash',
    name     : 'Bash',
    note     : 'The `/etc/bash_completion.d/` directory is the system-wide completion hook on '
             + 'most distributions. For a per-user install, write to '
             + '`~/.local/share/bash-completion/completions/prose` instead.',
    slug     : 'bash'
  },
  {
    caption  : 'use module',
    code     : `prose completions elvish > ~/.config/elvish/lib/prose-completions.elv
use prose-completions`,
    language : 'shellscript',
    name     : 'Elvish',
    note     : 'The `use` line goes in `~/.config/elvish/rc.elv` to register the completions '
             + 'on shell start.',
    slug     : 'elvish'
  },
  {
    caption  : 'completions directory',
    code     : `prose completions fish > ~/.config/fish/completions/prose.fish`,
    language : 'fish',
    name     : 'Fish',
    note     : 'Fish picks up completions in `~/.config/fish/completions/` on the next shell '
             + 'start, no `source` required.',
    slug     : 'fish'
  },
  {
    caption  : '$PROFILE script',
    code     : `if (!(Test-Path -Path $PROFILE.CurrentUserAllHosts)) { New-Item -ItemType File -Path $PROFILE.CurrentUserAllHosts -Force }
Add-Content -Path $PROFILE.CurrentUserAllHosts -Value '(& prose completions powershell) | Out-String | Invoke-Expression'`,
    language : 'powershell',
    name     : 'PowerShell',
    note     : 'PowerShell loads `$PROFILE.CurrentUserAllHosts` on every session, so the '
             + 'completions register on the next shell start.',
    slug     : 'powershell'
  },
  {
    caption  : 'fpath function',
    code     : `prose completions zsh > "\${fpath[1]}/_prose"`,
    language : 'zsh',
    name     : 'Zsh',
    note     : 'The `${fpath[1]}` expansion resolves to the first entry of zsh\'s function '
             + 'path, which is where `compinit` reads new completions from. Restart the shell '
             + 'or run `autoload -Uz compinit && compinit` to load the completions without '
             + 're-launching.',
    slug     : 'zsh'
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<readonly ShellCompletion[]> {
    const md = await renderer.getRenderer()
    return renderer.renderFencedField(md, renderer.inlineNodeField(md, SOURCES, 'note'), 'code')
  }
})
