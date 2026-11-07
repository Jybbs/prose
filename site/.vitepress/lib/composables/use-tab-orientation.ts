import { useMediaQuery }    from '@vueuse/core'
import { computed }         from 'vue'
import type { ComputedRef } from 'vue'

export function useTabOrientation(): ComputedRef<'horizontal' | 'vertical'> {
  // A numeric `ssrWidth` starts the client on the server's evaluation, which
  // reads `false` for a range query at any width, so the first client render
  // matches the server's `vertical` and the stacked layout lands as an update
  // after hydration rather than as a mismatch hydration leaves in place.
  const stacked = useMediaQuery('(width < 721px)', { ssrWidth: 1280 })

  return computed(() => stacked.value ? 'horizontal' : 'vertical')
}
