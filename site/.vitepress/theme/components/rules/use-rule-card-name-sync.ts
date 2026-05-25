import { type Ref, type WatchSource } from 'vue'

import { useMeasuredCssVar } from './use-measured-css-var'

export function useRuleCardNameSync(
  containerRef : Readonly<Ref<HTMLElement | null>>,
  source       : WatchSource
): void {
  useMeasuredCssVar({
    measure  : () => {
      const root = containerRef.value
      if (!root) return null
      let widest = 0
      for (const n of root.querySelectorAll<HTMLElement>('.rule-card-name')) {
        widest = Math.max(widest, n.scrollWidth)
      }
      return widest > 0 ? widest : null
    },
    propName : '--rule-card-name-width',
    target   : containerRef,
    triggers : [source]
  })
}
