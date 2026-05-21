import { useEventListener, useMounted, useResizeObserver } from '@vueuse/core'
import { onMounted, type Ref }                              from 'vue'

type TargetSource =
  | Ref<HTMLElement | null>
  | (() => HTMLElement | null | undefined)

export function useElementMeasure(measure: () => void, target: TargetSource): void {
  if (typeof window === 'undefined') return
  const mounted = useMounted()
  useResizeObserver(target, measure)
  useEventListener('resize', measure)
  onMounted(async () => {
    if ('fonts' in document) await document.fonts.ready
    if (mounted.value) requestAnimationFrame(measure)
  })
}
