import { useEventListener }                             from '@vueuse/core'
import { defineComponent, onBeforeUnmount, shallowRef } from 'vue'

import { Popup }    from '../../../kit'
import { RuleCard } from './RuleCard'

import { data as rules, type RenderedRule } from '../../../lib/rules/rules.data'

const HIDE_MS = 120

const SHOW_MS = 80

export const RuleLinkPopup = defineComponent({
  name: 'RuleLinkPopup',
  setup() {
    const rule   = shallowRef<RenderedRule | null>(null)
    const target = shallowRef<HTMLElement | null>(null)

    let timer: ReturnType<typeof setTimeout> | undefined

    // One delegated popup serves every `a[data-rule]` in the page, taking the
    // hovered anchor as its reference node.
    function linkFrom(event: Event): HTMLElement | null {
      const node = event.target
      return node instanceof Element ? node.closest<HTMLElement>('a[data-rule]') : null
    }

    // Reports whether an event landed on the open card, which keeps the card open
    // the way its link does.
    function onCard(event: Event): boolean {
      const node = event.target
      return node instanceof Element && node.closest('.rule-card-popup') !== null
    }

    function show(event: Event): void {
      if (onCard(event)) {
        clearTimeout(timer)
        return
      }
      const link = linkFrom(event)
      if (!link) return
      const found = rules.bySlug[link.dataset.rule ?? '']
      if (!found) return
      clearTimeout(timer)
      if (link === target.value && rule.value) return
      timer = setTimeout(() => {
        target.value = link
        rule.value   = found
      }, SHOW_MS)
    }

    function hide(event: Event): void {
      if (!linkFrom(event) && !onCard(event)) return
      clearTimeout(timer)
      timer = setTimeout(() => { rule.value = null }, HIDE_MS)
    }

    useEventListener('focusin',   show)
    useEventListener('focusout',  hide)
    useEventListener('mouseover', show)
    useEventListener('mouseout',  hide)

    onBeforeUnmount(() => clearTimeout(timer))

    return () => (
      <Popup
        anchor={target.value}
        class="rule-card-popup"
        label={rule.value?.name ?? ''}
        open={rule.value !== null}
        passive
        theme="rule-card"
      >
        {{ default: () => rule.value && <RuleCard clickable={false} rule={rule.value} /> }}
      </Popup>
    )
  }
})
