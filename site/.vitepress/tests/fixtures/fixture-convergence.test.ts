// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

vi.mock('../../lib/fixtures/fixtures.data', () => ({
  data: {
    band_constants: {
      already_settled: {
        changesSource : false,
        hasFindings   : false,
        hasToggle     : false,
        inputHtml     : '<pre><code>ZEBRA = 1</code></pre>',
        outputHtml    : '<pre><code>ZEBRA = 1</code></pre>'
      },
      hoists_a_constant: {
        changesSource : true,
        hasFindings   : false,
        hasToggle     : true,
        inputHtml     : '<pre><code>ZEBRA = 1</code></pre>',
        outputHtml    : '<pre><code>APPLE = 2</code></pre>'
      }
    }
  }
}))

import FixtureConvergence   from '../../theme/components/fixtures/FixtureConvergence.vue'
import { expectAccessible } from '../axe'

const mountCase = (name: string) =>
  mount(FixtureConvergence, { props: { case: name, rule: 'band_constants' } })

describe('FixtureConvergence', () => {
  it('tracks a rewriting fixture to a settled second run', () => {
    const w     = mountCase('hoists_a_constant')
    const steps = w.findAll('.fixture-converge-step')
    expect(steps.map(s => s.attributes('data-badge'))).toEqual([undefined, 'changed', 'settled'])
    expect(w.findAll('.fixture-converge-state')).toHaveLength(2)
    expect(w.get('[data-badge="settled"] .fixture-converge-note').text())
      .toContain('fixed point')
  })

  it('settles a fixed-point fixture on its first run', () => {
    const w     = mountCase('already_settled')
    const steps = w.findAll('.fixture-converge-step')
    expect(steps.map(s => s.attributes('data-badge'))).toEqual([undefined, 'settled'])
    expect(w.findAll('.fixture-converge-state')).toHaveLength(1)
    expect(w.text()).toContain('Run 1')
    expect(w.text()).not.toContain('Run 2')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountCase('hoists_a_constant').html())
  })
})
