// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import ReleaseToggle        from '../../theme/components/landing/ReleaseToggle.vue'
import { expectAccessible } from '../axe'

const mountToggle = (open: boolean) => mount(ReleaseToggle, { props: { open } })

describe('ReleaseToggle', () => {
  it('labels the closed state as opening the ledger', () => {
    const w = mountToggle(false)
    expect(w.text()).toBe('+')
    expect(w.attributes('aria-label')).toBe('Show previous releases')
    expect(w.attributes('aria-expanded')).toBe('false')
  })

  it('labels the open state as returning to the current release', () => {
    const w = mountToggle(true)
    expect(w.text()).toBe('←')
    expect(w.attributes('aria-label')).toBe('Back to current release')
    expect(w.attributes('aria-expanded')).toBe('true')
  })

  it('emits toggle on click', async () => {
    const w = mountToggle(false)
    await w.trigger('click')
    expect(w.emitted('toggle')).toHaveLength(1)
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountToggle(false).html())
  })
})
