import { vi } from 'vitest'

import type { DataStore, LoaderContext } from 'astro/loaders'

export type Entry = ReturnType<DataStore['values']>[number]

export interface Schema {
  parseAsync : (data: unknown) => Promise<Record<string, unknown>>
}

interface ContextInit {
  meta   ?: Iterable<[string, string]>
  root   ?: string
  schema ?: Schema
  store  ?: Iterable<Entry>
}

export interface FakeContext {
  ctx   : LoaderContext
  meta  : Map<string, string>
  store : Map<string, Entry>
  warn  : ReturnType<typeof vi.fn>
}

export function makeContext(init: ContextInit = {}): FakeContext {
  const meta  = new Map<string, string>(init.meta)
  const store = new Map<string, Entry>()
  for (const entry of init.store ?? []) store.set(entry.id, entry)
  const warn = vi.fn()

  const dataStore: DataStore = {
    addModuleImport : () => {},
    clear           : () => store.clear(),
    delete          : key => { store.delete(key) },
    entries         : () => [...store.entries()],
    get             : key => store.get(key) as never,
    has             : key => store.has(key),
    keys            : () => [...store.keys()],
    set             : entry => { store.set(entry.id, entry as Entry); return true },
    values          : () => [...store.values()]
  }

  const ctx = {
    collection     : 'test',
    config         : { root: new URL(init.root ?? 'file:///repo/site/') },
    generateDigest : (data: unknown) => JSON.stringify(data),
    logger         : { debug: () => {}, error: () => {}, info: () => {}, warn },
    meta           : {
      delete : (key: string) => { meta.delete(key) },
      get    : (key: string) => meta.get(key),
      has    : (key: string) => meta.has(key),
      set    : (key: string, value: string) => { meta.set(key, value) }
    },
    parseData      : async ({ data }: { data: Record<string, unknown> }) =>
      init.schema ? await init.schema.parseAsync(data) : data,
    renderMarkdown : () => Promise.resolve({ html: '' }),
    store          : dataStore
  } as unknown as LoaderContext

  return { ctx, meta, store, warn }
}
