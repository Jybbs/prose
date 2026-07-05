import { useMediaQuery } from '@vueuse/core'
import type { Ref }      from 'vue'

export function useReducedMotion(): Readonly<Ref<boolean>> {
  return useMediaQuery('(prefers-reduced-motion: reduce)')
}
