import { useResizeObserver }          from '@vueuse/core'
import { nextTick, onMounted, watch } from 'vue'
import type { Ref, WatchSource }      from 'vue'

import { snapInPlace } from '../shared/paint'

// Places one wash behind the element `selector` matches inside `grid`,
// sizing it to that element and copying its `data-layer`, each time `active`
// changes. The wash glides between places through its own transition, and it
// appears in place, through a `data-snap` that drops the transition for one
// write, on its first placement and whenever the grid's width changes.
export function useGlideWash(
  grid     : Readonly<Ref<HTMLElement | null>>,
  wash     : Readonly<Ref<HTMLElement | null>>,
  selector : string,
  active   : WatchSource<unknown>
): void {
  let width = 0

  const place = (): void => {
    const box    = grid.value?.getBoundingClientRect()
    const target = grid.value?.querySelector<HTMLElement>(selector)
    const el     = wash.value
    if (!box || !target || !el) return
    const at   = target.getBoundingClientRect()
    const snap = !('on' in el.dataset) || box.width !== width
    width = box.width
    snapInPlace(el, snap, () => {
      el.dataset.layer = target.dataset.layer
      Object.assign(el.style, {
        height    : `${at.height}px`,
        transform : `translate(${at.left - box.left}px, ${at.top - box.top}px)`,
        width     : `${at.width}px`
      })
      el.dataset.on = ''
    })
  }

  onMounted(place)
  watch(active, () => nextTick(place))
  useResizeObserver(grid, place)
}
