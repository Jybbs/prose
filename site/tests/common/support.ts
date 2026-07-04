import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { test as base, vi }                              from 'vitest'
import type { DetachedWindowAPI }                        from 'happy-dom'
import type { MockInstance }                             from 'vitest'

interface ContentEntry {
  data : unknown
  id   : string
}

interface Fixtures {
  container        : AstroContainer
  fakeRO           : { resize: (target: Element, rect: Partial<DOMRectReadOnly>) => void }
  loadFonts        : () => void
  mount            : (html: string) => HTMLElement
  setRect          : (target: Element, rect: Partial<DOMRect>) => void
  setReducedMotion : (value: 'no-preference' | 'reduce') => void
  warn             : MockInstance
}

export function astroContent(store: Record<string, ContentEntry[]>) {
  const of = (name: string): ContentEntry[] => store[name] ?? []
  return {
    getCollection : vi.fn(async (name: string, filter?: (entry: ContentEntry) => boolean) => of(name).filter(filter ?? (() => true))),
    getEntry      : vi.fn(async (name: string, id: string) => of(name).find(entry => entry.id === id))
  }
}

export const test = base.extend<Fixtures>({
  container: async ({}, use) => {
    await use(await AstroContainer.create())
  },

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

  mount: async ({}, use) => {
    const root = document.body.appendChild(document.createElement('div'))
    await use(html => { root.innerHTML = html; return root })
    root.remove()
  },

  setRect: async ({}, use) => {
    await use((target, rect) => Object.defineProperty(target, 'getBoundingClientRect', {
      configurable : true,
      value        : () => ({ bottom: 0, height: 0, left: 0, right: 0, toJSON() {}, top: 0, width: 0, x: 0, y: 0, ...rect })
    }))
  },

  setReducedMotion: async ({}, use) => {
    const { settings } = (window as unknown as { happyDOM: DetachedWindowAPI }).happyDOM
    await use(value => { settings.device.prefersReducedMotion = value })
    settings.device.prefersReducedMotion = 'no-preference'
  },

  warn: async ({}, use) => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    await use(spy)
    spy.mockRestore()
  }
})
