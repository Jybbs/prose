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

  it('opens on the whole rule set, its command naming every rule', () => {
    expect(render().get('p code').text()).toBe('prose format --select a,b,c')
  })

  it('strikes each step whose output matches the step before it', () => {
    expect(render().findAll('.composition-trail-step').map(step => step.attributes('data-still') !== undefined))
      .toStrictEqual([false, true, false, true])
  })

  it('shows the case as written from the first step', async () => {
    const wrapper = render()
    await wrapper.findAll('[role=tab]')[0].trigger('click')
    expect(wrapper.get('p').text()).toBe('The case as written, before any rule runs')
    expect(wrapper.findAll('.grid > div').find(panel => !isInert(panel))?.text()).toBe('in')
  })
})
