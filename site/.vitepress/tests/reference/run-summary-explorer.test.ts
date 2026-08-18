// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import RunSummaryExplorer   from '../../theme/components/reference/RunSummaryExplorer.vue'
import { expectAccessible } from '../axe'
import { popperStubMount }  from '../popper-stub'

const mountExplorer = () => mount(RunSummaryExplorer, { global: popperStubMount })

describe('RunSummaryExplorer', () => {
  it('opens on the clean check run', () => {
    const w = mountExplorer()
    expect(w.get('.run-summary-msg').text()).toBe('All clean.')
    expect(w.get('.run-summary-anchor').text()).toBe('🪻')
    expect(w.get('.run-summary-caption').text()).toBe('A clean run, full output, on a tty.')
  })

  it('re-renders the line and gloss when an outcome is picked', async () => {
    const w = mountExplorer()
    await w.findAll('.run-summary-opt')[1].trigger('click')
    expect(w.get('.run-summary-msg').text()).toBe('5 diagnostics in 2 files.')
    expect(w.get('.run-summary-caption').text()).toBe('Violations found, full output, on a tty.')
  })

  it('drops the anchor under --quiet', async () => {
    const w     = mountExplorer()
    const quiet = w
      .findAll('.run-summary-opt')
      .find(opt => opt.get('.run-summary-opt-mono').text() === '--quiet')

    expect(quiet).toBeDefined()
    await quiet!.trigger('click')

    expect(w.find('.run-summary-anchor').exists()).toBe(false)
    expect(w.get('.run-summary-msg').text()).toBe('All clean.')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountExplorer().html())
  })
})
