// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { ExitCodeMatrix }    from '../../theme/components/exit-codes/ExitCodeMatrix'
import { rendersAccessibly } from '../axe'

vi.mock('../../lib/exit-codes/exit-codes.data', () => {
  const labels = ['Returned by', 'CI gate', 'Next step', 'Note']
  const facts  = (returned: string) => [returned, 'Fails.', 'Fix it.', 'A note.']
    .map((text, at) => ({ label: labels[at], nodes: [{ kind: 'text', text }] }))
  return { data: [
    { band: 'Passes',   code: 0, facts: facts('Both subcommands.'), label: 'Clean',         outcome: 'success',  summary: 'No diagnostics.' },
    { band: 'Findings', code: 1, facts: facts('A rewrite.'),        label: 'Would change',  outcome: 'findings', summary: 'A rewrite is pending.' },
    { band: 'Errors',   code: 3, facts: facts('A parse failure.'),  label: 'Parse failure', outcome: 'error',    summary: 'A file failed to parse.' }
  ] }
})

const mountMatrix = () => mount(ExitCodeMatrix)

describe('ExitCodeMatrix', () => {
  it('opens on the first exit code, carrying its outcome', () => {
    const w = mountMatrix()
    expect(w.get('article').attributes('data-outcome')).toBe('success')
    expect(w.get('.exit-code-entry-numeral').text()).toBe('0')
    expect(w.get('.exit-code-entry-label').text()).toBe('Clean')
  })

  it('opens a band for each outcome on its first code', () => {
    const bands = mountMatrix().findAll('.exit-codes-band-head')
    expect(bands.map(band => [band.attributes('data-outcome'), band.text()]))
      .toStrictEqual([['success', 'Passes'], ['findings', 'Findings'], ['error', 'Errors']])
  })

  it('lists the code\'s facts as ruled rows of a label beside its text', () => {
    const facts = mountMatrix().get('.exit-code-entry-facts')
    expect(facts.findAll('dt').map(dt => dt.text())).toStrictEqual(['Returned by', 'CI gate', 'Next step', 'Note'])
    expect(facts.findAll('dd').map(dd => dd.text())).toStrictEqual(['Both subcommands.', 'Fails.', 'Fix it.', 'A note.'])
  })

  it('swaps the entry when another row takes the pointer', async () => {
    const w = mountMatrix()
    await w.findAll('.exit-code-index-row')[2]!.trigger('mouseenter')
    expect(w.get('article').attributes('data-outcome')).toBe('error')
    expect(w.get('.exit-code-entry-summary').text()).toBe('A file failed to parse.')
  })

  it('swaps the entry when another row takes focus', async () => {
    const w = mountMatrix()
    await w.findAll('.exit-code-index-row')[2]!.trigger('focus')
    expect(w.get('.exit-code-entry-label').text()).toBe('Parse failure')
  })

  rendersAccessibly(() => mountMatrix().html())
})
