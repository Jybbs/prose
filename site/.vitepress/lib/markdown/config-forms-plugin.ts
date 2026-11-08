import type { MarkdownIt } from 'markdown-it'

// The fence info a configuration block opens on, written in its `prose.toml`
// form.
const CONFIG_INFO = 'toml config'

const HEADER = /^(\s*\[\[?)(?=[\w"'-][^\]]*\]\]?\s*(?:#.*)?$)/

// Rewrites a `prose.toml` body as the `[tool.prose]` table of a
// `pyproject.toml`, prefixing each table header and opening any keys above the
// first header under `[tool.prose]`.
export function pyprojectForm(toml: string): string {
  const lines  = toml.trimEnd().split('\n')
  const first  = lines.findIndex(line => HEADER.test(line))
  const rooted = lines.slice(0, first === -1 ? undefined : first).some(line => /^\s*[^\s#]/.test(line))
  return [...rooted ? ['[tool.prose]'] : [], ...lines.map(line => line.replace(HEADER, '$1tool.prose.'))].join('\n')
}

// Sets each `toml config` fence as a code group holding its `prose.toml` and
// `pyproject.toml` forms, inside the `ConfigForms` component that loads the
// code-group styles and icons on the pages using one.
export function configFormsPlugin(md: MarkdownIt): void {
  md.core.ruler.after('block', 'config-forms', state => {
    state.tokens = state.tokens.flatMap(token => {
      if (token.type !== 'fence' || token.info.trim() !== CONFIG_INFO) return [token]
      const fence = (title: string, body: string): string => `\`\`\`toml [${title}]\n${body}\n\`\`\``
      const group = [
        '::: code-group',
        fence('prose.toml', token.content.trimEnd()),
        fence('pyproject.toml', pyprojectForm(token.content)),
        ':::'
      ].join('\n')
      const html = (content: string): InstanceType<typeof state.Token> =>
        Object.assign(new state.Token('html_block', '', 0), { content: `${content}\n` })
      return [html('<ConfigForms>'), ...state.md.parse(group, state.env), html('</ConfigForms>')]
    })
  })
}
