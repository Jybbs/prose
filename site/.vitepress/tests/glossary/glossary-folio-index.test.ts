// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import GlossaryFolioIndex   from '../../theme/components/glossary/GlossaryFolioIndex.vue'
import { useGlossaryFolio } from '../../lib/composables/use-glossary-folio'

vi.mock('vitepress', () => ({
  useData: () => ({ page: { value: { relativePath: 'reference/glossary.md' } } })
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

const index  = () => mount(GlossaryFolioIndex)
const raised = (w: ReturnType<typeof index>) =>
  w.findAll('.glossary-folio-row.is-raised').map(row => row.text())

describe('GlossaryFolioIndex', () => {
  it('keeps every row lowered at rest, though an entry is selected', () => {
    const w = index()
    expect(w.get('.glossary-folio-index').attributes('data-raised')).toBeUndefined()
    expect(raised(w)).toStrictEqual([])
  })

  it('raises every row naming the family a reader points at', async () => {
    const w = index()
    await w.get('[data-family="ordering"]').trigger('mouseenter')
    expect(w.get('.glossary-folio-index').attributes('data-raised')).toBe('ordering')
    expect(raised(w)).toStrictEqual(['pipeline', 'sort'])
  })

  it('selects a clicked row and pins its primary family', async () => {
    const w = index()
    await w.get('[data-family="engine"]').trigger('click')
    expect(useGlossaryFolio().selected.value).toBe('pipeline')
    expect(w.get('.glossary-folio-index').attributes('data-raised')).toBe('engine')
    expect(raised(w)).toStrictEqual(['pipeline'])
  })
})
