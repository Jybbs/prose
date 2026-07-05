// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import ShellCompletions     from '../../theme/components/integrations/ShellCompletions.vue'
import { expectAccessible } from '../axe'

vi.mock('../../data/shell-completions.data', () => ({
  data: [
    {
      caption  : 'completions bash',
      codeHtml : '<pre>prose completions bash</pre>',
      name     : 'Bash',
      noteHtml : '<p>Reload the shell.</p>',
      slug     : 'bash'
    },
    {
      caption  : 'completions fish',
      codeHtml : '<pre>prose completions fish</pre>',
      name     : 'fish',
      noteHtml : '<p>Completions load lazily.</p>',
      slug     : 'fish'
    }
  ]
}))

const mountCard = () => mount(ShellCompletions, { global: { stubs: { Tool: true } } })

describe('ShellCompletions', () => {
  it('opens on the first shell', () => {
    const w = mountCard()
    expect(w.findAll('.shell-card-tab')).toHaveLength(2)
    expect(w.findAll('.shell-card-tab')[0].attributes('aria-selected')).toBe('true')
    expect(w.get('.shell-card-modal .kicker').text()).toBe('completions bash')
  })

  it('swaps the modal when another tab is clicked', async () => {
    const w = mountCard()
    await w.findAll('.shell-card-tab')[1].trigger('click')
    expect(w.findAll('.shell-card-tab')[1].attributes('aria-selected')).toBe('true')
    expect(w.findAll('.shell-card-tab')[0].attributes('aria-selected')).toBe('false')
    expect(w.get('.shell-card-modal .kicker').text()).toBe('completions fish')
    expect(w.get('.shell-card-modal-note').text()).toBe('Completions load lazily.')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountCard().html())
  })
})
