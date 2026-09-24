// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import DirectiveAnatomy      from '../../theme/components/suppression/DirectiveAnatomy.vue'
import InlineProse           from '../../theme/components/base/InlineProse.vue'
import { SCOPE_ORDER }       from '../../lib/suppression/scopes'
import { rendersAccessibly } from '../axe'

vi.mock('../../lib/suppression/directives.data', async () => {
  const { directiveParts } = await import('../../lib/suppression/directive-parts')
  const records = [
    {
      effectNodes : [{ kind: 'text', text: 'Suppresses every rewrite for the file.' }],
      example     : '# prose: off',
      form        : '# prose: off',
      id          : 'prose-off',
      scope       : 'file'
    },
    {
      effectNodes : [{ kind: 'text', text: 'Opens a suppressed region.' }],
      example     : '# fmt: off',
      form        : '# fmt: off',
      id          : 'fmt-off',
      pairId      : 'fmt-on',
      pairRole    : 'opens',
      scope       : 'block'
    },
    {
      effectNodes : [{ kind: 'text', text: 'Only the listed lint rules skip the line.' }],
      example     : 'x = 1  # prose: ignore[<rule>]',
      form        : '# prose: ignore[<rule>, ...]',
      id          : 'prose-ignore-rules',
      scope       : 'line'
    },
    {
      effectNodes : [{ kind: 'text', text: 'Keeps the dict entries in authored order.' }],
      example     : 'config = {}  # prose: keep',
      form        : '# prose: keep',
      id          : 'prose-keep',
      scope       : 'construct'
    }
  ]
  return { data: records.map(directive => ({ ...directive, parts: directiveParts(directive.form) })) }
})

const mountAnatomy = () => mount(DirectiveAnatomy, { global: { components: { InlineProse } } })

describe('DirectiveAnatomy', () => {
  it('renders one band per scope in the shared order', () => {
    const bands = mountAnatomy().findAll('.directive-anatomy-band')
    expect(bands.map(b => b.attributes('data-scope'))).toStrictEqual([...SCOPE_ORDER])
  })

  it('seeds the focus on the bracketed ignore directive', () => {
    const w = mountAnatomy()
    expect(w.get('[data-active="true"]').text()).toBe('# prose: ignore[<rule>, ...]')
    expect(w.findAll('.directive-anatomy-part').map(p => p.text()))
      .toStrictEqual(['#', 'prose:', 'ignore', '[<rule>, ...]'])
  })

  it('swaps the plate to the clicked directive', async () => {
    const w = mountAnatomy()
    await w.get('[data-scope="construct"] .directive-anatomy-thumb').trigger('click')
    expect(w.get('.directive-anatomy-plate').attributes('data-scope')).toBe('construct')
    expect(w.get('.directive-anatomy-effect').text()).toBe('Keeps the dict entries in authored order.')
  })

  rendersAccessibly(() => mountAnatomy().html())
})
