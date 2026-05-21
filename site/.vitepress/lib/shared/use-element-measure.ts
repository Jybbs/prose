import { useEventListener, useResizeObserver } from '@vueuse/core'
import { onMounted, type Ref }                 from 'vue'

type TargetSource =
  | Ref<HTMLElement | null>
  | (() => HTMLElement | null | undefined)

export function useElementMeasure(measure: () => void, target: TargetSource): void {
  if (typeof window === 'undefined') return
  useResizeObserver(target, measure)
  useEventListener('resize', measure)
  onMounted(() => {
    if ('fonts' in document) document.fonts.ready.then(() => requestAnimationFrame(measure))
    requestAnimationFrame(measure)
  })
}
