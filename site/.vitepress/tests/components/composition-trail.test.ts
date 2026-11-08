// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { CompositionTrail } from '../../theme/components/rules/CompositionTrail'
import { isInert }          from '../dom'

const { frontmatter } = vi.hoisted(() => ({
  frontmatter: {
    value: {
      fixtureTrails : { 'composition/alpha_case': { outputs: ['<pre>in</pre>', '<pre>out</pre>'], rules: ['a', 'b', 'c'], steps: [0, 0, 1, 1] } },
      fixtures      : { 'composition/alpha_case': { inputHtml: '<pre>in</pre>' } }
    }
  }
}))

vi.mock('vitepress', async importOriginal => ({
  ...(await importOriginal<typeof import('vitepress')>()),
  useData: () => ({ frontmatter })
}))

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub())

describe('CompositionTrail', () => {
  const render = () => mount(CompositionTrail, { props: { case: 'alpha_case' } })

  const shown = (wrapper: ReturnType<typeof render>) =>
    wrapper.findAll('p > span').filter(line => !line.classes('invisible'))

  it('opens on the whole rule set, its command naming every rule', () => {
    expect(shown(render()).map(line => line.text())).toStrictEqual(['prose format --select a,b,c'])
  })

  it('names the rules up to the chosen step', async () => {
    const wrapper = render()
    await wrapper.findAll('[role=tab]')[2].trigger('click')
    expect(shown(wrapper).map(line => line.text())).toStrictEqual(['prose format --select a,b'])
  })

  it('stacks every step line in one cell, hiding all but the shown one from a reader', () => {
    const lines = render().findAll('p > span')
    expect(lines).toHaveLength(4)
    expect(lines.map(line => line.attributes('aria-hidden'))).toStrictEqual(['true', 'true', 'true', undefined])
  })

  it('strikes each step whose output matches the step before it', () => {
    expect(render().findAll('.composition-trail-step').map(step => step.attributes('data-still') !== undefined))
      .toStrictEqual([false, true, false, true])
  })

  it('shows the case as written from the first step', async () => {
    const wrapper = render()
    await wrapper.findAll('[role=tab]')[0].trigger('click')
    expect(shown(wrapper).map(line => line.text())).toStrictEqual(['The case as written, before any rule runs'])
    expect(wrapper.findAll('.grid > div').find(panel => !isInert(panel))?.text()).toBe('in')
  })
})
