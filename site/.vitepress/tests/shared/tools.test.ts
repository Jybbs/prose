import editorConfigs    from '../../lib/integrations/editor-configs.data'
import shellCompletions from '../../lib/integrations/shell-completions.data'
import { lookup }       from '../../lib/shared/lookup'
import loader           from '../../lib/shared/tools.data'

const { entries } = await loader.load([])

const CARD_SLUGS = [...await editorConfigs.load([]), ...await shellCompletions.load([])]
  .map(({ slug }) => slug)

describe('the tools loader', () => {
  it.each(CARD_SLUGS)('draws the %s mark in the current color alone', slug => {
    const { body } = lookup(entries, slug, 'Card mark').icon
    expect(body).toContain('fill="currentColor"')
    expect(body).not.toMatch(/fill="(?!currentColor")|#[0-9a-f]{3,8}\b|url\(|stroke="/i)
  })

  it('draws the terminal mark from its local asset, on its own view box', () => {
    expect(entries.elvish.icon.viewBox).toBe('18 5 73 73')
    expect(entries.elvish.icon.body).toContain('<g fill="currentColor">')
  })
})
