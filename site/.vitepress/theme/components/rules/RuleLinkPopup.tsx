import { useEventListener, useTimeoutFn } from '@vueuse/core'
import { defineComponent, shallowRef }    from 'vue'

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

    const reveal  = useTimeoutFn((link: HTMLElement, found: RenderedRule) => {
      target.value = link
      rule.value   = found
    }, SHOW_MS, { immediate: false })
    const conceal = useTimeoutFn(() => { rule.value = null }, HIDE_MS, { immediate: false })

    function hold(): void {
      reveal.stop()
      conceal.stop()
    }

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
        hold()
        return
      }
      const link = linkFrom(event)
      if (!link) return
      const found = rules.bySlug[link.dataset.rule ?? '']
      if (!found) return
      hold()
      if (link === target.value && rule.value) return
      reveal.start(link, found)
    }

    function hide(event: Event): void {
      if (!linkFrom(event) && !onCard(event)) return
      hold()
      conceal.start()
    }

    useEventListener('focusin',   show)
    useEventListener('focusout',  hide)
    useEventListener('mouseover', show)
    useEventListener('mouseout',  hide)

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
