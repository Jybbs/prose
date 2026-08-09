import { createInjectionState }                                       from '@vueuse/core'
import { computed, toValue, type ComputedRef, type MaybeRefOrGetter } from 'vue'

export const [provideAriaHidden, useAriaHidden] = createInjectionState(
  (hidden: MaybeRefOrGetter<boolean>) => computed(() => toValue(hidden)),
  { defaultValue: computed(() => false) }
)

// The tabindex a focusable node takes inside a hidden subtree, `undefined`
// leaving the node's natural place in the tab order.
export function useHiddenTabindex(): ComputedRef<-1 | undefined> {
  const hidden = useAriaHidden()
  return computed(() => (hidden.value ? -1 : undefined))
}
