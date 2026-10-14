// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { computed } from 'vue'

import GlossaryFolioPane from '../../theme/components/glossary/GlossaryFolioPane.vue'
import { railPaint }     from '../../lib/shared/family-rail'

vi.mock('../../lib/composables/use-glossary-folio', () => ({
  useGlossaryFolio: () => {
    const entry = {
      aliases         : [],
      definitionNodes : [],
      families        : ['formatting', 'alignment', 'ordering'],
      href            : '/reference/glossary#annotation',
      primaryFamily   : 'formatting',
      slug            : 'annotation'
    }
    return {
      active      : computed(() => entry),
      activeIndex : computed(() => 0),
      filtered    : computed(() => [entry]),
      step        : () => {}
    }
  }
}))

vi.mock('../../lib/composables/route', () => ({ useCurrentRule: () => ({ value: null }) }))

const pane = () => mount(GlossaryFolioPane)

describe('GlossaryFolioPane', () => {
  it('paints the pane\'s rail through every family the entry belongs to, in order', () => {
    expect(pane().get('.glossary-folio-stage').attributes('style'))
      .toContain(railPaint(['formatting', 'alignment', 'ordering']))
  })

  it('heads the entry with one unlinked family chip per family, in order', () => {
    const chips = pane().findAll('.glossary-folio-folio .family-chip')
    expect(chips.map(chip => [chip.attributes('data-family'), chip.attributes('href')]))
      .toStrictEqual([['formatting', undefined], ['alignment', undefined], ['ordering', undefined]])
  })
})
