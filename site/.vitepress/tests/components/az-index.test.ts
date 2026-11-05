// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import AzIndex from '../../theme/components/reference/AzIndex.vue'

vi.mock('../../lib/tokens/tokens.data', () => ({
  data: ['check', 'diff'].map(sort => ({
    blurbNodes  : [],
    destination : { title: 'CLI' },
    domain      : 'cli-flag',
    href        : '/reference/cli',
    key         : `--${sort}`,
    sort
  }))
}))

describe('AzIndex', () => {
  it('marks only the key its card describes as current', async () => {
    const wrapper         = mount(AzIndex)
    const [check, diff]   = wrapper.findAll('a.az-index-key-btn')
    expect(check!.attributes('aria-current')).toBeUndefined()
    await check!.trigger('mouseenter')
    await flushPromises()
    expect(check!.attributes('aria-current')).toBe('true')
    expect(diff!.attributes('aria-current')).toBeUndefined()
  })
})
