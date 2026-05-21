export interface ToolSeed {
  href : string
  icon : string
  name : string
  role : string
}

export const TOOL_SEEDS = {
  clap: {
    href : 'https://docs.rs/clap/',
    icon : 'logos:rust',
    name : 'clap',
    role : 'CLI parsing'
  },
  emacs: {
    href : 'https://www.gnu.org/software/emacs/',
    icon : 'logos:emacs',
    name : 'Emacs',
    role : 'Editor integration target'
  },
  github: {
    href : 'https://github.com/features/actions',
    icon : 'logos:github-actions',
    name : 'GitHub Actions',
    role : 'CI integration target'
  },
  helix: {
    href : 'https://helix-editor.com/',
    icon : 'simple-icons:helix',
    name : 'Helix',
    role : 'Editor integration target'
  },
  jetbrains: {
    href : 'https://www.jetbrains.com/',
    icon : 'logos:intellij-idea',
    name : 'JetBrains',
    role : 'Editor integration target'
  },
  maturin: {
    href : 'https://www.maturin.rs/',
    icon : 'logos:rust',
    name : 'maturin',
    role : 'Rust to Python wheel build'
  },
  mise: {
    href : 'https://mise.jdx.dev/',
    icon : 'custom:mise',
    name : 'mise',
    role : 'Tool versions and tasks'
  },
  neovim: {
    href : 'https://neovim.io/',
    icon : 'logos:neovim',
    name : 'Neovim',
    role : 'Editor integration target'
  },
  precommit: {
    href : 'https://pre-commit.com/',
    icon : 'simple-icons:precommit',
    name : 'pre-commit',
    role : 'Git commit-boundary hook'
  },
  python: {
    href : 'https://www.python.org/',
    icon : 'logos:python',
    name : 'Python',
    role : 'The target language'
  },
  rayon: {
    href : 'https://docs.rs/rayon/',
    icon : 'logos:rust',
    name : 'rayon',
    role : 'Per-file parallelism'
  },
  ruff: {
    href : 'https://docs.astral.sh/ruff/',
    icon : 'simple-icons:ruff',
    name : 'Ruff',
    role : 'Token-level upstream pass'
  },
  rust: {
    href : 'https://www.rust-lang.org/',
    icon : 'logos:rust',
    name : 'Rust',
    role : 'Implementation language'
  },
  sublime: {
    href : 'https://www.sublimetext.com/',
    icon : 'logos:sublimetext-icon',
    name : 'Sublime Text',
    role : 'Editor integration target'
  },
  uv: {
    href : 'https://docs.astral.sh/uv/',
    icon : 'simple-icons:uv',
    name : 'uv',
    role : 'Canonical install path'
  },
  vitepress: {
    href : 'https://vitepress.dev/',
    icon : 'simple-icons:vitepress',
    name : 'VitePress',
    role : 'Docs site framework'
  },
  vscode: {
    href : 'https://code.visualstudio.com/',
    icon : 'logos:visual-studio-code',
    name : 'VS Code',
    role : 'Editor integration target'
  }
} as const satisfies Record<string, ToolSeed>

export type ToolSlug = keyof typeof TOOL_SEEDS
