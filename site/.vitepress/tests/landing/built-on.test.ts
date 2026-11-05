// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import BuiltOn               from '../../theme/components/landing/BuiltOn.vue'
import { rendersAccessibly } from '../axe'

import type loader from '../../lib/shared/tools.data'

vi.mock('../../lib/shared/tools.data', async importOriginal =>
  ({ data: await (await importOriginal<{ default: typeof loader }>()).default.load([]) }))

describe('BuiltOn', () => {
  it('links each credit to its tool with its mark in a medallion', () => {
    const w     = mount(BuiltOn)
    const hrefs = w.findAll('a.built-on-card').map(card => card.attributes('href'))
    expect(hrefs).toContain('https://mise.jdx.dev/')
    expect(w.findAll('a.built-on-card .medallion svg')).toHaveLength(hrefs.length)
  })

  rendersAccessibly(() => mount(BuiltOn).html())
})
