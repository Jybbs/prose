// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { RunSummaryExplorer } from '../../theme/components/reference/RunSummaryExplorer'
import * as runSummary        from '../../lib/reference/run-summary'
import { rendersAccessibly }  from '../axe'

const mountExplorer = () => mount(RunSummaryExplorer)

describe('RunSummaryExplorer', () => {
  it('opens on the clean check run', () => {
    const w = mountExplorer()
    expect(w.get('.run-summary-msg').text()).toBe('All clean.')
    expect(w.get('.run-summary-anchor').text()).toBe('🪻')
    expect(w.get('.run-summary-caption').text()).toBe('A clean run, full output, on a tty.')
  })

  it('re-renders the line and gloss when an outcome is picked', async () => {
    const w = mountExplorer()
    await w.findAll('.run-summary-select-trigger')[0].trigger('click')
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
    await w.findAll('.run-summary-select-trigger')[1].trigger('click')
    await quiet!.trigger('click')

    expect(w.get('.run-summary-msg').element.parentElement!.querySelector('.run-summary-anchor')).toBeNull()
    expect(w.get('.run-summary-msg').text()).toBe('All clean.')
  })

  it('holds the longest outcome in a hidden line behind the one shown', () => {
    const reserve = mountExplorer().get('[aria-hidden="true"].invisible')
    expect(reserve.get('.run-summary-msg').text()).toBe(runSummary.longestLine().text)
  })

  rendersAccessibly(() => mountExplorer().html())
})
