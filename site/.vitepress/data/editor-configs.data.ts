import { createMarkdownRenderer, defineLoader } from 'vitepress'

import { siteDir } from '../lib/shared/paths'

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
    language : 'lisp',
    name     : 'Emacs',
    slug     : 'emacs',
    target   : 'init.el',
    code     : `;; Add to ~/.emacs.d/init.el
(add-hook 'after-save-hook
  (lambda ()
    (when (eq major-mode 'python-mode)
      (call-process "prose" nil nil nil "format" buffer-file-name))))`
  },
  {
    caption  : 'editor.formatter',
    language : 'toml',
    name     : 'Helix',
    slug     : 'helix',
    target   : 'languages.toml',
    code     : `[[editor.formatter]]
languages = ["python"]
command   = "prose"
args      = ["format", "-"]`
  },
  {
    caption  : 'File Watchers',
    language : 'text',
    name     : 'JetBrains',
    slug     : 'jetbrains',
    target   : 'Watcher dialog',
    code     : `File type        : Python
Scope            : Project Files
Program          : prose
Arguments        : format $FilePath$
Working directory: $ProjectFileDir$`
  },
  {
    caption  : 'autocmd BufWritePost',
    language : 'vim',
    name     : 'Neovim',
    slug     : 'neovim',
    target   : 'init.vim',
    code     : `autocmd BufWritePost *.py silent! !prose format %`
  },
  {
    caption  : 'SublimeOnSaveBuild',
    language : 'python',
    name     : 'Sublime Text',
    slug     : 'sublime',
    target   : '<Project>.sublime-project',
    code     : `# Install: SublimeOnSaveBuild
# Add to <Project>.sublime-project:
{
  "build_systems": [{
    "name"        : "prose",
    "shell_cmd"   : "prose format \\"$file\\"",
    "selector"    : "source.python",
    "working_dir" : "$file_path"
  }]
}`
  },
  {
    caption  : 'emeraldwalk.runonsave',
    language : 'json',
    name     : 'VS Code',
    slug     : 'vscode',
    target   : 'settings.json',
    code     : `{
  "emeraldwalk.runonsave": {
    "commands": [
      {
        "match": "\\\\.py$",
        "cmd"  : "prose format \${file}"
      }
    ]
  }
}`
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<readonly EditorConfig[]> {
    const md = await createMarkdownRenderer(siteDir(import.meta.url))
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
