// @vitest-environment happy-dom
import { flushPromises } from '@vue/test-utils'
import { ref }           from 'vue'

import { useRuleCardNameSync }           from '../../lib/composables/use-rule-card-name-sync'
import { domTest, mountSetup, nextFrame } from '../dom'

const cardName = (scrollWidth: number): HTMLElement => {
  const el = document.createElement('span')
  el.className = 'rule-card-name'
  Object.defineProperty(el, 'scrollWidth', { value: scrollWidth })
  return el
}

describe('useRuleCardNameSync', () => {
  domTest('sets the width prop to the widest rule-card-name', async ({ fonts }) => {
    const root = document.createElement('div')
    root.append(cardName(60), cardName(88))
    const target = ref<HTMLElement | null>(root)
    mountSetup(() => useRuleCardNameSync(target, () => 0))
    fonts.settle()
    await flushPromises()
    await nextFrame()
    expect(root.style.getPropertyValue('--rule-card-name-width')).toBe('88px')
  })

  domTest('leaves the prop unset when no rule-card-name is present', async ({ fonts }) => {
    const root   = document.createElement('div')
    const target = ref<HTMLElement | null>(root)
    mountSetup(() => useRuleCardNameSync(target, () => 0))
    fonts.settle()
    await flushPromises()
    await nextFrame()
    expect(root.style.getPropertyValue('--rule-card-name-width')).toBe('')
  })
})
