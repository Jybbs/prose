// oxlint-disable no-empty-pattern -- vitest fixtures require object destructuring
import { mount }              from '@vue/test-utils'
import { test as base }       from 'vitest'
import { defineComponent, h } from 'vue'

import type { DOMWrapper }        from '@vue/test-utils'
import type { DetachedWindowAPI } from 'happy-dom'

interface DomFixtures {
  fonts          : { settle: () => void }
  reducedMotion  : (matches: boolean) => void
  resizeObserver : { fire: () => void }
}

class FakeResizeObserver implements ResizeObserver {
  static latest: FakeResizeObserver | undefined
  callback: ResizeObserverCallback

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback
    FakeResizeObserver.latest = this
  }

  disconnect(): void {}
  observe(): void {}
  unobserve(): void {}
}

// `reducedMotion` drives the device setting `matchMedia` reads at mount, so
// call it before mounting.
export const domTest = base.extend<DomFixtures>({
  fonts: async ({}, use) => {
    let settle!: () => void
    const ready = new Promise<void>(resolve => { settle = resolve })
    Object.defineProperty(document, 'fonts', { configurable: true, value: { ready } })
    await use({ settle })
    delete (document as { fonts?: unknown }).fonts
  },
  reducedMotion: async ({}, use) => {
    const device = (window as unknown as { happyDOM: DetachedWindowAPI }).happyDOM.settings.device
    const prior  = device.prefersReducedMotion
    await use(matches => {
      device.prefersReducedMotion = matches ? 'reduce' : 'no-preference'
    })
    device.prefersReducedMotion = prior
  },
  resizeObserver: async ({}, use) => {
    const prior = globalThis.ResizeObserver
    globalThis.ResizeObserver = FakeResizeObserver
    await use({
      fire: () => {
        const latest = FakeResizeObserver.latest
        latest?.callback([], latest)
      }
    })
    FakeResizeObserver.latest = undefined
    globalThis.ResizeObserver = prior
  }
})

// happy-dom leaves `isVisible` truthy for a `v-show`-hidden element, so
// visibility reads the inline style the directive writes.
export const isHidden = (element: Pick<DOMWrapper<Element>, 'attributes'>): boolean =>
  element.attributes('style')?.includes('display: none') ?? false

export const mountSetup = <T>(run: () => T): T => {
  let api!: T
  mount(defineComponent({
    setup() {
      api = run()
      return () => h('div')
    }
  }))
  return api
}

export const nextFrame = (): Promise<void> =>
  new Promise(resolve => { requestAnimationFrame(() => resolve()) })

export const rectElement = (rect: Partial<DOMRect>): HTMLElement => {
  const el = document.createElement('div')
  el.getBoundingClientRect = () => rect as DOMRect
  return el
}
