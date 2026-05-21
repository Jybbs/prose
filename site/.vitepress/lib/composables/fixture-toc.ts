import { inject, provide, ref, type InjectionKey, type Ref } from 'vue'

export interface FixtureTocEntry {
  id    : string
  rule  : string
  title : string
}

const FIXTURE_TOC_KEY: InjectionKey<Ref<FixtureTocEntry[]>> = Symbol('fixtureToc')

export function provideFixtureToc(): Ref<FixtureTocEntry[]> {
  const entries = ref<FixtureTocEntry[]>([])
  provide(FIXTURE_TOC_KEY, entries)
  return entries
}

export function useFixtureToc(): Ref<FixtureTocEntry[]> {
  return inject(FIXTURE_TOC_KEY) ?? ref([])
}
