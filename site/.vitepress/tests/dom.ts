// oxlint-disable no-empty-pattern -- vitest fixtures require object destructuring
import { mount }                   from '@vue/test-utils'
import { test as base }            from 'vitest'
import { defineComponent, h, ref } from 'vue'

import type { DOMWrapper }        from '@vue/test-utils'
import type { DetachedWindowAPI } from 'happy-dom'

import type { ProseSandbox } from '../lib/composables/use-prose-sandbox'

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

// Builds a whole `ProseSandbox` at rest, which a component test overrides
// with only the fields its case moves and reads without a cast.
export const fakeSandbox = (overrides: Partial<ProseSandbox> = {}): ProseSandbox => ({
  configError  : ref(''),
  configToml   : ref(''),
  diagnostics  : ref([]),
  drawn        : ref(0),
  eligible     : ref([]),
  error        : ref(''),
  facetImpact  : ref({}),
  facetValue   : (_slug, facet) => facet.default,
  formatNow    : () => {},
  formatted    : ref(''),
  lengthImpact : ref(null),
  lengths      : [],
  lengthValue  : () => 88,
  refresh      : () => {},
  rules        : [],
  setFacet     : () => {},
  setLength    : () => {},
  share        : () => Promise.resolve(null),
  source       : ref(''),
  start        : () => Promise.resolve(),
  unstable     : ref([]),
  ...overrides
})

// happy-dom's `checkVisibility` reports false for a detached element, and
// these mounts render detached, so visibility reads the inline style the
// `v-show` directive writes.
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

export const rectElement = (rect: Partial<DOMRect>): HTMLElement =>
  stubRect(document.createElement('div'), rect)

export const stubRect = <T extends Element>(element: T, rect: Partial<DOMRect>): T => {
  element.getBoundingClientRect = () => rect as DOMRect
  return element
}
