// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { OutputSamples } from '../../theme/components/reference/OutputSamples'
import { isInert }       from '../dom'

vi.mock('../../lib/reference/output-samples.data', () => ({
  data: {
    diffHtml  : '<pre>+++</pre>',
    findings  : [
      { code: 'align-equals', column: 2, inputHtml: '<pre data-rule="align-equals"></pre>', row: 1 },
      { code: 'band-constants', column: 1, inputHtml: '<pre data-rule="band-constants"></pre>', row: 1 }
    ],
    inputHtml : '<pre></pre>',
    records   : { github: ['<p>a</p>', '<p>b</p>'], json: ['<p>a</p>', '<p>b</p>'], text: ['<p>a</p>', '<p>b</p>'] },
    summary   : '<pre>summary</pre>'
  }
}))

describe('OutputSamples', () => {
  it('marks the first finding in each format and draws its squiggle', () => {
    const wrapper = mount(OutputSamples)
    expect(wrapper.findAll('[data-marked]')).toHaveLength(3)
    expect(wrapper.html()).toContain('data-rule="align-equals"')
  })

  it('moves the mark and the squiggle to the finding chosen in the index', async () => {
    const wrapper = mount(OutputSamples)
    const buttons = wrapper.findAll('button[aria-pressed]')
    await buttons[1].trigger('click')
    expect(buttons[1].attributes('aria-pressed')).toBe('true')
    expect(wrapper.html()).toContain('data-rule="band-constants"')
    expect(wrapper.findAll('[data-marked]').map(node => node.text())).toStrictEqual(['b', 'b', 'b'])
  })

  it('keeps every format in the grid, leaving the hidden ones inert', () => {
    const panels = mount(OutputSamples).findAll('.grid > div')
    expect(panels).toHaveLength(4)
    expect(panels.filter(panel => !isInert(panel))).toHaveLength(1)
  })
})
