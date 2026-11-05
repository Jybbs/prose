// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import GlossaryFolioIndex   from '../../theme/components/glossary/GlossaryFolioIndex.vue'
import { useGlossaryFolio } from '../../lib/composables/use-glossary-folio'
import { declared }         from '../support'

vi.mock('vitepress', () => ({
  defineLoader : () => {},
  useData      : () => ({ page: { value: { relativePath: 'reference/glossary.md' } } })
}))

vi.mock('../../lib/composables/use-glossary-folio', async () => {
  const { computed, ref } = await import('vue')
  const entries = [
    { families: ['alignment'],          primaryFamily: 'alignment', slug: 'column' },
    { families: ['engine', 'ordering'], primaryFamily: 'engine',    slug: 'pipeline' },
    { families: ['ordering'],           primaryFamily: 'ordering',  slug: 'sort' }
  ]
  const folio = {
    filtered : computed(() => entries),
    grouped  : computed(() => entries.map(entry => [entry.slug[0]!.toUpperCase(), [entry]])),
    ordered  : entries,
    query    : ref(''),
    selected : ref('column')
  }
  return { useGlossaryFolio: () => folio }
})

const index = () => mount(GlossaryFolioIndex, { attachTo: document.body })

const decks = (w: ReturnType<typeof index>) => w.findAll('.glossary-folio-row').map(row => [
  row.get('.glossary-folio-row-slug').text(),
  row.findAll('.glossary-folio-row-tile').map(tile => tile.attributes('data-family')),
  row.find('.glossary-folio-row-deck').classes('is-stacked')
])

describe('GlossaryFolioIndex', () => {
  it('ends each row in its term\'s family badges, stacking several as a deck', () => {
    expect(decks(index())).toStrictEqual([
      ['column',   ['alignment'],          false],
      ['pipeline', ['engine', 'ordering'], true],
      ['sort',     ['ordering'],           false]
    ])
  })

  it('stacks the first family on top, each tile beneath it one place further in', () => {
    const tiles = index().findAll('.glossary-folio-row-deck.is-stacked .glossary-folio-row-tile')
    expect(tiles.map(tile => tile.attributes('style'))).toStrictEqual(['--deck-at: 0;', '--deck-at: 1;'])
  })

  it('holds the deck\'s width on the resting step, so a spread moves no name', () => {
    const sheet = `${import.meta.dirname}/../../theme/components/glossary/styles/glossary-folio-index.css`
    expect(declared(sheet, '.glossary-folio-row-deck').width).not.toContain('--deck-step')
  })

  it('marks the selected row and selects a clicked one', async () => {
    const w = index()
    expect(w.findAll('.glossary-folio-row[aria-current]').map(row => row.text())).toStrictEqual(['column'])
    await w.findAll('.glossary-folio-row')[1]!.trigger('click')
    expect(useGlossaryFolio().selected.value).toBe('pipeline')
  })

  it.each([
    ['ArrowDown', 0, 1],
    ['ArrowUp',   1, 0],
    ['End',       0, 2],
    ['Home',      2, 0],
    ['ArrowUp',   0, 0]
  ])('moves the focus under %s from row %i to row %i', async (key, from, to) => {
    const rows = index().findAll('.glossary-folio-row')
    await rows[from]!.trigger('keydown', { key })
    expect(document.activeElement).toBe(rows[to]!.element)
  })

  it('leaves the focus where it stands under another key', async () => {
    const row = index().findAll<HTMLElement>('.glossary-folio-row')[1]!
    row.element.focus()
    await row.trigger('keydown', { key: 'Enter' })
    expect(document.activeElement).toBe(row.element)
  })
})
