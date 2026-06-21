import { computed, shallowRef, type ComputedRef, type Ref } from 'vue'

export function useTabSelect<T, K>(
  items : readonly T[],
  keyOf : (item: T) => K
): { active: ComputedRef<T>; selected: Ref<K> } {
  const selected = shallowRef(keyOf(items[0]))
  const active   = computed(() => items.find(i => keyOf(i) === selected.value) ?? items[0])
  return { active, selected }
}
