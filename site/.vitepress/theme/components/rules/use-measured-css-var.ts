import { watch, type Ref, type WatchSource } from 'vue'

import { useSettledMeasure } from '../../../lib/composables/use-settled-measure'

interface UseMeasuredCssVarOptions {
  measure   : () => number | null
  observe  ?: Readonly<Ref<HTMLElement | null>>
  propName  : string
  target    : Readonly<Ref<HTMLElement | null>>
  triggers ?: WatchSource[]
}

export function useMeasuredCssVar(options: UseMeasuredCssVarOptions): void {
  const { measure, observe, propName, target, triggers } = options
  let measuring = false

  function run() {
    const el = target.value
    if (measuring || !el) return
    measuring = true
    el.style.removeProperty(propName)

    requestAnimationFrame(() => {
      const t = target.value
      if (!t) { measuring = false; return }
      const value = measure()
      if (value !== null && value > 0) {
        t.style.setProperty(propName, `${Math.ceil(value)}px`)
      }
      measuring = false
    })
  }

  useSettledMeasure(observe ?? target, run)
  if (triggers) for (const source of triggers) watch(source, run)
}
