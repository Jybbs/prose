import { converter }            from 'culori'
import { chromium, webkit }     from 'playwright'
import { inject, test as base } from 'vitest'

import type { Browser, BrowserContext, BrowserContextOptions, Locator, Page } from 'playwright'

import type { Scheme } from '../../lib/shared/palette'

interface Visit {
  engine? : Engine
  motion? : NonNullable<BrowserContextOptions['reducedMotion']>
  scheme? : Scheme
  width?  : number
}

export const ENGINES = { chromium, webkit }

export const PAGE_WIDTH = 1440

export type Engine = keyof typeof ENGINES

const COLOR = /\b(?:color|lab|lch|oklab|oklch|rgba?)\([^)]*\)/g

// Sets the OKLCH chroma past which a color reads as a hue, above each VitePress
// text color and its shades and below each role and each other palette shade.
const NEUTRAL_CHROMA = 0.02

const oklch = converter('oklch')

// Returns each color in the computed `value` that reads as a hue.
export const huesIn = (value: string): string[] =>
  (value.match(COLOR) ?? []).filter(color => oklch(color)!.c > NEUTRAL_CHROMA)

// Moves the pointer onto `target` and waits out the transitions it starts.
export async function pointAt(target: Locator): Promise<void> {
  await target.hover()
  await settle(target.page())
}

// Launches each engine's browser once per test file, on the first visit that
// names it. Each `visit` opens its route in a new context at the given engine,
// motion preference, color scheme, and viewport width, and resolves once the
// page's root has hydrated.
export const renderTest = base
  // oxlint-disable-next-line no-empty-pattern -- vitest fixtures require object destructuring
  .extend('browsers', { scope: 'file' }, ({}, { onCleanup }) => {
    const launched = {} as Partial<Record<Engine, Promise<Browser>>>
    onCleanup(async () => {
      await Promise.all(Object.values(launched).map(async browser => (await browser).close()))
    })
    return (engine: Engine): Promise<Browser> => (launched[engine] ??= ENGINES[engine].launch())
  })
  .extend('visit', ({ browsers }, { onCleanup }) => {
    const contexts = [] as BrowserContext[]
    onCleanup(async () => { await Promise.all(contexts.map(context => context.close())) })
    return async (
      route: string,
      { engine = 'chromium', motion = 'no-preference', scheme = 'light', width = PAGE_WIDTH }: Visit = {}
    ): Promise<Page> => {
      const browser = await browsers(engine)
      const context = await browser.newContext({
        colorScheme   : scheme,
        reducedMotion : motion,
        viewport      : { height: 900, width }
      })
      contexts.push(context)
      context.setDefaultTimeout(10_000)
      const page  = await context.newPage()
      const reply = await page.goto(new URL(route, inject('siteUrl')).href)
      expect(reply?.ok()).toBe(true)
      await page.waitForFunction(() => '__vue_app__' in document.querySelector('#app')!)
      return page
    }
  })

// Waits until every animation and transition on the page has finished.
export async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await Promise.allSettled(document.getAnimations().map(({ finished }) => finished))
  })
}

// Reads the value the browser computes for `prop` on `target` or its `pseudo`.
export const styleOf = (target: Locator, prop: string, pseudo?: string): Promise<string> =>
  target.evaluate(
    (element, [name, pseudoElement]) => getComputedStyle(element, pseudoElement).getPropertyValue(name),
    [prop, pseudo] as const
  )
