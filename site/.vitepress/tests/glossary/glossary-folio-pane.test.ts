// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { computed } from 'vue'

import GlossaryFolioPane from '../../theme/components/glossary/GlossaryFolioPane.vue'

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
  it('sets the pane\'s headword in the link style', () => {
    expect(pane().get('.glossary-folio-headword a').classes()).toContain('body-link')
  })

  it('heads the entry with one unlinked family chip per family, in order', () => {
    const chips = pane().findAll('.glossary-folio-folio .family-chip')
    expect(chips.map(chip => [chip.attributes('data-family'), chip.attributes('href')]))
      .toStrictEqual([['formatting', undefined], ['alignment', undefined], ['ordering', undefined]])
  })
})
