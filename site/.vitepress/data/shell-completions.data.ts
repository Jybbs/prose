import { defineLoader } from 'vitepress'

import { getRenderer, renderFencedHtml, renderInlineField } from '../lib/markdown/renderer'

interface ShellCompletion {
  caption  : string
  codeHtml : string
  command  : string
  language : string
  mono     : string
  name     : string
  noteHtml : string
  slug     : string
  target   : string
}

declare const data: readonly ShellCompletion[]
export { data }

interface ShellCompletionSource {
  caption  : string
  code     : string
  command  : string
  language : string
  mono     : string
  name     : string
  note     : string
  slug     : string
  target   : string
}

const SOURCES: readonly ShellCompletionSource[] = [
  {
    caption  : 'bash_completion.d',
    code     : `prose completions bash > /etc/bash_completion.d/prose`,
    command  : 'prose completions bash',
    language : 'bash',
    mono     : 'bash',
    name     : 'Bash',
    note     : 'The `/etc/bash_completion.d/` directory is the system-wide completion hook on most distributions. For a per-user install, write to `~/.local/share/bash-completion/completions/prose` instead.',
    slug     : 'bash',
    target   : '/etc/bash_completion.d/prose'
  },
  {
    caption  : 'use module',
    code     : `prose completions elvish > ~/.config/elvish/lib/prose-completions.elv
use prose-completions`,
    command  : 'prose completions elvish',
    language : 'shellscript',
    mono     : 'elvish',
    name     : 'Elvish',
    note     : 'The `use` line goes in `~/.config/elvish/rc.elv` to register the completions on shell start.',
    slug     : 'elvish',
    target   : '~/.config/elvish/lib/prose-completions.elv'
  },
  {
    caption  : 'completions directory',
    code     : `prose completions fish > ~/.config/fish/completions/prose.fish`,
    command  : 'prose completions fish',
    language : 'fish',
    mono     : 'fish',
    name     : 'Fish',
    note     : 'Fish picks up completions in `~/.config/fish/completions/` on the next shell start, no `source` required.',
    slug     : 'fish',
    target   : '~/.config/fish/completions/prose.fish'
  },
  {
    caption  : '$PROFILE script',
    code     : `prose completions powershell > $PROFILE.CurrentUserAllHosts`,
    command  : 'prose completions powershell',
    language : 'powershell',
    mono     : 'powershell',
    name     : 'PowerShell',
    note     : 'PowerShell loads `$PROFILE.CurrentUserAllHosts` on every session, so the completions register on the next shell start.',
    slug     : 'powershell',
    target   : '$PROFILE.CurrentUserAllHosts'
  },
  {
    caption  : 'fpath function',
    code     : `prose completions zsh > "\${fpath[1]}/_prose"`,
    command  : 'prose completions zsh',
    language : 'zsh',
    mono     : 'zsh',
    name     : 'Zsh',
    note     : 'The `${fpath[1]}` expansion lands at the first entry of zsh\'s function path, which is where `compinit` picks up new completions. Restart the shell or run `autoload -Uz compinit && compinit` to pick the completions up without re-launching.',
    slug     : 'zsh',
    target   : '${fpath[1]}/_prose'
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<readonly ShellCompletion[]> {
    const md = await getRenderer()
    const withNote = renderInlineField(md, SOURCES, 'note')
    return withNote.map(({ code, note, ...rest }) => ({
      ...rest,
      codeHtml : renderFencedHtml(md, code, rest.language)
    }))
  }
})
