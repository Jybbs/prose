import { inject, provide, ref, type InjectionKey } from 'vue'

export interface FixtureTocEntry {
  id    : string
  rule  : string
  title : string
}

export interface FixtureTocApi {
  get      : (rule: string) => readonly FixtureTocEntry[]
  register : (entry: FixtureTocEntry) => () => void
}

const FIXTURE_TOC_KEY: InjectionKey<FixtureTocApi> = Symbol('fixtureToc')

export function provideFixtureToc(): FixtureTocApi {
  const buckets = ref<Map<string, FixtureTocEntry[]>>(new Map())
  const api: FixtureTocApi = {
    get: rule => buckets.value.get(rule) ?? [],
    register(entry) {
      const list = [...(buckets.value.get(entry.rule) ?? []), entry]
      buckets.value.set(entry.rule, list)
      return () => {
        const next = buckets.value.get(entry.rule)?.filter(e => e !== entry) ?? []
        if (next.length) buckets.value.set(entry.rule, next)
        else buckets.value.delete(entry.rule)
      }
    }
  }
  provide(FIXTURE_TOC_KEY, api)
  return api
}

export function useFixtureToc(): FixtureTocApi {
  return inject(FIXTURE_TOC_KEY) ?? {
    get      : () => [],
    register : () => () => {}
  }
}
