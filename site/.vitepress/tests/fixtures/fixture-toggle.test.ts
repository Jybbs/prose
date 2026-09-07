// @vitest-environment happy-dom
import { mount, type VueWrapper } from '@vue/test-utils'

import FixtureNoChange      from '../../theme/components/fixtures/FixtureNoChange.vue'
import FixtureToggle        from '../../theme/components/fixtures/FixtureToggle.vue'
import { expectAccessible } from '../axe'

import type { FixtureTab } from '../../lib/shared/fixture-tab'

const mountToggle = (tab: FixtureTab) => mount(FixtureToggle, { props: { modelValue: tab } })

const pills = (w: VueWrapper) =>
  w.findAll('.fixture-toggle-pill').map(p => [p.attributes('data-side'), p.text()])

describe('FixtureToggle', () => {
  it('marks the modeled side selected', () => {
    const w = mountToggle('after')
    expect(w.get('[data-side="after"]').attributes('aria-selected')).toBe('true')
    expect(w.get('[data-side="before"]').attributes('aria-selected')).toBe('false')
  })

  it('emits the clicked side', async () => {
    const w = mountToggle('before')
    await w.get('[data-side="after"]').trigger('click')
    expect(w.emitted('update:modelValue')).toEqual([['after']])
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountToggle('before').html())
  })
})

describe('FixtureNoChange', () => {
  it('reserves the toggle footprint with matching pills', () => {
    expect(pills(mount(FixtureNoChange))).toEqual(pills(mountToggle('after')))
  })
})
