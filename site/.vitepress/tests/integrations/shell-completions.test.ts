// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { ToolIcon }          from '../../kit'
import { ShellCompletions }  from '../../theme/components/integrations/ShellCompletions'
import { rendersAccessibly } from '../axe'

import type loader from '../../lib/shared/tools.data'

vi.mock('../../lib/shared/tools.data', async importOriginal =>
  ({ data: await (await importOriginal<{ default: typeof loader }>()).default.load([]) }))

vi.mock('../../lib/integrations/shell-completions.data', () => ({
  data: [
    {
      caption   : 'completions bash',
      codeHtml  : '<pre>prose completions bash</pre>',
      name      : 'Bash',
      noteNodes : [{ kind: 'text', text: 'Reload the shell.' }],
      slug      : 'bash'
    },
    {
      caption   : 'completions fish',
      codeHtml  : '<pre>prose completions fish</pre>',
      name      : 'fish',
      noteNodes : [{ kind: 'text', text: 'Completions load lazily.' }],
      slug      : 'fish'
    }
  ]
}))

describe('ShellCompletions', () => {
  it('opens on the first shell', () => {
    const w = mount(ShellCompletions)
    expect(w.findAll('.shell-card-tab')).toHaveLength(2)
    expect(w.findAll('.shell-card-tab')[0].attributes('aria-selected')).toBe('true')
    expect(w.get('.shell-card-modal .kicker').text()).toBe('completions bash')
  })

  it('sets each tab\'s mark and the modal\'s in a ring holding no link', () => {
    const w = mount(ShellCompletions)
    expect(w.findAll('.tool-ring svg')).toHaveLength(3)
    expect(w.find('.tool-ring a').exists()).toBe(false)
    expect(w.findAllComponents(ToolIcon).map(icon => icon.props('slug')))
      .toStrictEqual(['bash', 'fish', 'bash'])
  })

  it('draws each tab in gray at rest and the selected one on the page with a Ube edge', () => {
    const tab = mount(ShellCompletions).get('.shell-card-tab')
    expect(tab.classes()).toStrictEqual(expect.arrayContaining([
      'bg-bg-soft',
      'border-divider',
      'group',
      'text-text-2',
      'aria-selected:bg-bg',
      'aria-selected:border-ube',
      'aria-selected:text-text-1'
    ]))
    expect(tab.get('.tool-ring').classes()).toContain('group-aria-selected:text-link')
  })

  it('swaps the modal when another tab takes the pointer', async () => {
    const w = mount(ShellCompletions)
    await w.findAll('.shell-card-tab')[1].trigger('mouseenter')
    expect(w.findAll('.shell-card-tab')[1].attributes('aria-selected')).toBe('true')
    expect(w.findAll('.shell-card-tab')[0].attributes('aria-selected')).toBe('false')
    expect(w.get('.shell-card-modal .kicker').text()).toBe('completions fish')
    expect(w.get('.shell-card-modal').getComponent(ToolIcon).props('slug')).toBe('fish')
    expect(w.get('.shell-card-modal-note').text()).toBe('Completions load lazily.')
  })

  it('swaps the modal when another tab takes focus', async () => {
    const w = mount(ShellCompletions)
    await w.findAll('.shell-card-tab')[1].trigger('focus')
    expect(w.get('.shell-card-modal .kicker').text()).toBe('completions fish')
  })

  rendersAccessibly(() => mount(ShellCompletions).html())
})
