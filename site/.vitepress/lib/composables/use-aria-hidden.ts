import { createInjectionState }                     from '@vueuse/core'
import { computed, toValue, type MaybeRefOrGetter } from 'vue'

export const [provideAriaHidden, useAriaHidden] = createInjectionState(
  (hidden: MaybeRefOrGetter<boolean>) => computed(() => toValue(hidden)),
  { defaultValue: computed(() => false) }
)
