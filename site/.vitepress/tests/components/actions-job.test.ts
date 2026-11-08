// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { ActionsJob } from '../../theme/components/integrations/ActionsJob'

vi.mock('../../lib/integrations/actions-job.data', async () => {
  const job = await import('../../lib/integrations/actions-job')
  const formats = ['text', 'sarif']
  return {
    data: {
      formats,
      workflows: Object.fromEntries(job.everyChoice(formats).map(choice => [job.choiceKey(choice), `<pre>${job.choiceKey(choice)}</pre>`]))
    }
  }
})

describe('ActionsJob', () => {
  it('shows the default workflow until a token is chosen', () => {
    const shown = mount(ActionsJob).findAll('.flush-code-block').at(-1)!
    expect(shown.text()).toBe('text')
  })

  it('adds a step for each pressed token and swaps the format', async () => {
    const wrapper = mount(ActionsJob)
    const [cache] = wrapper.findAll('button[aria-pressed]')
    await cache.trigger('click')
    await wrapper.findAll('[role=tab]')[1].trigger('click')
    expect(cache.attributes('aria-pressed')).toBe('true')
    expect(wrapper.findAll('.flush-code-block').at(-1)!.text()).toBe('sarif+cache')
  })
})
