import { createMarkdownRenderer } from 'vitepress'

import { configFormsPlugin, pyprojectForm } from '../../lib/markdown/config-forms-plugin'
import { siteDir }                          from '../../lib/shared/paths'

const md = await createMarkdownRenderer(siteDir(import.meta.url), {
  config: renderer => { renderer.use(configFormsPlugin) }
})

describe('pyprojectForm', () => {
  it('opens root keys under `[tool.prose]` and prefixes every table header', () => {
    expect(pyprojectForm('a = 1\n\n[rules]\nb = false\n\n[[overrides]]\npaths = ["x"]\n'))
      .toBe('[tool.prose]\na = 1\n\n[tool.prose.rules]\nb = false\n\n[[tool.prose.overrides]]\npaths = ["x"]')
  })

  it('adds no root header to a body opening on a table', () => {
    expect(pyprojectForm('[cache]\nenabled = true')).toBe('[tool.prose.cache]\nenabled = true')
  })

  it('adds no root header above a table that only a comment precedes', () => {
    expect(pyprojectForm('# cache\n[cache]\nenabled = true')).toBe('# cache\n[tool.prose.cache]\nenabled = true')
  })

  it('prefixes an indented header and leaves a value line opening on `[` as written', () => {
    expect(pyprojectForm('[rules]\n  [rules.align-equals]\nfirst-party = [\n["a", "b"],\n]'))
      .toBe('[tool.prose.rules]\n  [tool.prose.rules.align-equals]\nfirst-party = [\n["a", "b"],\n]')
  })
})

describe('configFormsPlugin', () => {
  it('sets a `toml config` fence as both forms inside `ConfigForms`', async () => {
    const html = await md.renderAsync('```toml config\n[cache]\nenabled = true\n```\n')
    expect(html).toMatch(/^<ConfigForms>/)
    expect(html).toContain('vp-code-group')
    expect(html).toContain('prose.toml')
    expect(html).toContain('pyproject.toml')
    expect(html.replaceAll(/<[^>]+>/g, '')).toContain('[tool.prose.cache]')
  })

  it('leaves any other fence as written', async () => {
    const html = await md.renderAsync('```toml\n[cache]\n```\n')
    expect(html).not.toContain('ConfigForms')
  })
})
