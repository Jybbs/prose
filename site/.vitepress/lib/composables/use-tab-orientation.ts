import { useMediaQuery }    from '@vueuse/core'
import { computed }         from 'vue'
import type { ComputedRef } from 'vue'

export function useTabOrientation(): ComputedRef<'horizontal' | 'vertical'> {
  const stacked = useMediaQuery('(width < 721px)', { ssrWidth: 1280 })

  return computed(() => stacked.value ? 'horizontal' : 'vertical')
}
