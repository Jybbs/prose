// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import ExitCodeMatrix       from '../../theme/components/exit-codes/ExitCodeMatrix.vue'
import { expectAccessible } from '../axe'

vi.mock('../../data/exit-codes.data', () => ({
  data: [
    { code: 0, detailHtml: ['<code>prose check</code> passes.'], label: 'Clean',         summary: 'No diagnostics.' },
    { code: 3, detailHtml: ['<em>Parse</em> stops the run.'],    label: 'Parse failure', summary: 'A file failed to parse.' }
  ]
}))

const mountMatrix = () => mount(ExitCodeMatrix)

describe('ExitCodeMatrix', () => {
  it('opens on the first exit code', () => {
    const w = mountMatrix()
    expect(w.get('article').attributes('data-exit-code')).toBe('0')
    expect(w.get('.exit-code-entry-label').text()).toBe('Clean')
  })

  it('swaps the entry when another row takes the pointer', async () => {
    const w = mountMatrix()
    await w.findAll('.exit-code-index-row')[1].trigger('mouseenter')
    expect(w.get('article').attributes('data-exit-code')).toBe('3')
    expect(w.get('.exit-code-entry-summary').text()).toBe('A file failed to parse.')
  })

  it('swaps the entry when another row takes focus', async () => {
    const w = mountMatrix()
    await w.findAll('.exit-code-index-row')[1].trigger('focus')
    expect(w.get('.exit-code-entry-label').text()).toBe('Parse failure')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountMatrix().html())
  })
})
