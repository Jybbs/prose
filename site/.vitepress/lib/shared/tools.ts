// Declares one tool the `Tool` chip links to, `language` marking a name that
// also names a language, whose chip marks only a mention of the interpreter or
// the toolchain.
interface ToolSeed {
  href      : string
  icon      : string
  language ?: true
  name      : string
}

export const TOOL_SEEDS = {
  bash: {
    href : 'https://www.gnu.org/software/bash/',
    icon : 'simple-icons:gnubash',
    name : 'Bash'
  },
  clap: {
    href : 'https://docs.rs/clap/',
    icon : 'logos:rust',
    name : 'clap'
  },
  elvish: {
    href : 'https://elv.sh/',
    icon : 'custom:terminal',
    name : 'Elvish'
  },
  emacs: {
    href : 'https://www.gnu.org/software/emacs/',
    icon : 'simple-icons:gnuemacs',
    name : 'Emacs'
  },
  fish: {
    href : 'https://fishshell.com/',
    icon : 'simple-icons:fishshell',
    name : 'Fish'
  },
  github: {
    href : 'https://github.com/features/actions',
    icon : 'simple-icons:githubactions',
    name : 'GitHub Actions'
  },
  helix: {
    href : 'https://helix-editor.com/',
    icon : 'simple-icons:helix',
    name : 'Helix'
  },
  jetbrains: {
    href : 'https://www.jetbrains.com/',
    icon : 'simple-icons:jetbrains',
    name : 'JetBrains'
  },
  maturin: {
    href : 'https://www.maturin.rs/',
    icon : 'logos:rust',
    name : 'maturin'
  },
  mise: {
    href : 'https://mise.jdx.dev/',
    icon : 'custom:mise',
    name : 'mise'
  },
  neovim: {
    href : 'https://neovim.io/',
    icon : 'simple-icons:neovim',
    name : 'Neovim'
  },
  powershell: {
    href : 'https://learn.microsoft.com/powershell/',
    icon : 'simple-icons:powershell',
    name : 'PowerShell'
  },
  precommit: {
    href : 'https://pre-commit.com/',
    icon : 'simple-icons:precommit',
    name : 'pre-commit'
  },
  python: {
    href     : 'https://www.python.org/',
    icon     : 'logos:python',
    language : true,
    name     : 'Python'
  },
  rayon: {
    href : 'https://docs.rs/rayon/',
    icon : 'logos:rust',
    name : 'rayon'
  },
  ruff: {
    href : 'https://docs.astral.sh/ruff/',
    icon : 'simple-icons:ruff',
    name : 'Ruff'
  },
  rust: {
    href     : 'https://www.rust-lang.org/',
    icon     : 'simple-icons:rust',
    language : true,
    name     : 'Rust'
  },
  sublime: {
    href : 'https://www.sublimetext.com/',
    icon : 'simple-icons:sublimetext',
    name : 'Sublime Text'
  },
  uv: {
    href : 'https://docs.astral.sh/uv/',
    icon : 'simple-icons:uv',
    name : 'uv'
  },
  vitepress: {
    href : 'https://vitepress.dev/',
    icon : 'simple-icons:vitepress',
    name : 'VitePress'
  },
  vscode: {
    href : 'https://code.visualstudio.com/',
    icon : 'simple-icons:visualstudiocode',
    name : 'VS Code'
  },
  zsh: {
    href : 'https://www.zsh.org/',
    icon : 'simple-icons:zsh',
    name : 'Zsh'
  }
} as const satisfies Record<string, ToolSeed>

export type ToolSlug = keyof typeof TOOL_SEEDS
