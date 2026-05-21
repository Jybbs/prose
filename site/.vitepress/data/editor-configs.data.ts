import { defineLoader } from 'vitepress'

import { getRenderer } from '../lib/markdown/renderer'

export interface EditorConfig {
  caption  : string
  codeHtml : string
  language : string
  name     : string
  slug     : string
  target   : string
}

declare const data: readonly EditorConfig[]
export { data }

interface EditorConfigSource {
  caption  : string
  code     : string
  language : string
  name     : string
  slug     : string
  target   : string
}

const SOURCES: readonly EditorConfigSource[] = [
  {
    caption  : 'after-save-hook',
    code     : `;; Add to ~/.emacs.d/init.el
(add-hook 'after-save-hook
  (lambda ()
    (when (eq major-mode 'python-mode)
      (call-process "prose" nil nil nil "format" buffer-file-name))))`,
    language : 'lisp',
    name     : 'Emacs',
    slug     : 'emacs',
    target   : 'init.el'
  },
  {
    caption  : 'editor.formatter',
    code     : `[[editor.formatter]]
languages = ["python"]
command   = "prose"
args      = ["format", "-"]`,
    language : 'toml',
    name     : 'Helix',
    slug     : 'helix',
    target   : 'languages.toml'
  },
  {
    caption  : 'File Watchers',
    code     : `File type        : Python
Scope            : Project Files
Program          : prose
Arguments        : format $FilePath$
Working directory: $ProjectFileDir$`,
    language : 'text',
    name     : 'JetBrains',
    slug     : 'jetbrains',
    target   : 'Watcher dialog'
  },
  {
    caption  : 'autocmd BufWritePost',
    code     : `autocmd BufWritePost *.py silent! !prose format %`,
    language : 'vim',
    name     : 'Neovim',
    slug     : 'neovim',
    target   : 'init.vim'
  },
  {
    caption  : 'SublimeOnSaveBuild',
    code     : `# Install: SublimeOnSaveBuild
# Add to <Project>.sublime-project:
{
  "build_systems": [{
    "name"        : "prose",
    "shell_cmd"   : "prose format \\"$file\\"",
    "selector"    : "source.python",
    "working_dir" : "$file_path"
  }]
}`,
    language : 'python',
    name     : 'Sublime Text',
    slug     : 'sublime',
    target   : '<Project>.sublime-project'
  },
  {
    caption  : 'emeraldwalk.runonsave',
    code     : `{
  "emeraldwalk.runonsave": {
    "commands": [
      {
        "match": "\\\\.py$",
        "cmd"  : "prose format \${file}"
      }
    ]
  }
}`,
    language : 'json',
    name     : 'VS Code',
    slug     : 'vscode',
    target   : 'settings.json'
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<readonly EditorConfig[]> {
    const md = await getRenderer()
    return SOURCES.map(src => ({
      caption  : src.caption,
      codeHtml : md.render(`\`\`\`${src.language}\n${src.code}\n\`\`\``),
      language : src.language,
      name     : src.name,
      slug     : src.slug,
      target   : src.target
    }))
  }
})
