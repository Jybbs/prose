import { test as base, vi }       from 'vitest'
import type { DetachedWindowAPI } from 'happy-dom'

interface ContentEntry {
  data : unknown
  id   : string
}

interface Fixtures {
  fakeRO           : { resize: (target: Element, rect: Partial<DOMRectReadOnly>) => void }
  loadFonts        : () => void
  setReducedMotion : (value: 'no-preference' | 'reduce') => void
}

export function astroContent(store: Record<string, ContentEntry[]>) {
  const of = (name: string): ContentEntry[] => store[name] ?? []
  return {
    getCollection : vi.fn(async (name: string, filter?: (entry: ContentEntry) => boolean) => of(name).filter(filter ?? (() => true))),
    getEntry      : vi.fn(async (name: string, id: string) => of(name).find(entry => entry.id === id))
  }
}

export const test = base.extend<Fixtures>({
  fakeRO: async ({}, use) => {
    const callbacks = new Map<Element, ResizeObserverCallback>()
    class FakeResizeObserver {
      constructor(private readonly callback: ResizeObserverCallback) {}
      disconnect() { callbacks.clear() }
      observe(target: Element) { callbacks.set(target, this.callback) }
      unobserve(target: Element) { callbacks.delete(target) }
    }
    vi.stubGlobal('ResizeObserver', FakeResizeObserver)
    await use({
      resize(target, rect) {
        callbacks.get(target)?.(
          [{ contentRect: { height: 0, width: 0, ...rect }, target } as ResizeObserverEntry],
          {} as ResizeObserver
        )
      }
    })
    vi.unstubAllGlobals()
  },

  loadFonts: async ({}, use) => {
    let settle: () => void
    const ready = new Promise<void>(resolve => { settle = resolve })
    Object.defineProperty(document, 'fonts', {
      configurable : true,
      value        : { ready }
    })
    await use(() => settle())
  },

  setReducedMotion: async ({}, use) => {
    const { settings } = (window as unknown as { happyDOM: DetachedWindowAPI }).happyDOM
    await use(value => { settings.device.prefersReducedMotion = value })
    settings.device.prefersReducedMotion = 'no-preference'
  }
})
