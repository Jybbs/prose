import axe          from 'axe-core'
import { chromium } from 'playwright'
import type { Browser, BrowserContext, Page, ViewportSize } from 'playwright'

import { SCHEMES }     from '../../lib/shared/palette'
import type { Scheme } from '../../lib/shared/palette'

import { BUILT, bundleContext, PAGES } from './bundle'

interface Finding {
  id      : string
  targets : string[]
}

interface Layout {
  overflow : number
  split    : string[]
  stranded : string[]
}

interface Scrolling {
  regions     : Finding[]
  unreachable : string[]
}

interface Survey {
  axe      : Record<Scheme, Finding[]>
  layout   : Record<number, Layout>
  overflow : Scrolling
}

const FOCUSABLE       = 'a[href], button:not([disabled]), input, select, textarea, [tabindex]:not([tabindex="-1"])'
const FORCED_OVERFLOW = 'pre, code, table, td { letter-spacing: 0.5em; white-space: nowrap; }'
const NARROW          : ViewportSize = { height: 800, width: 360 }
const OVERFLOWING     : ViewportSize = { height: 640, width: 320 }
const WIDE            : ViewportSize = { height: 800, width: 1280 }

// Runs `axe-core` on the page, limited to the rules `runOnly` names where it names any.
async function audit(page: Page, runOnly?: string[]): Promise<Finding[]> {
  await page.addScriptTag({ content: axe.source })
  return page.evaluate(async options => {
    const { violations } = await (window as unknown as { axe: typeof axe }).axe.run(document, options)
    return violations.map(({ id, nodes }) => ({ id, targets: nodes.map(node => node.target.join(' ')) }))
  }, runOnly ? { runOnly } : {})
}

// Reads how far the page runs past the viewport's width, each inline code chip
// whose box splits across lines or carries the page past that width, and each chip
// whose following punctuation opens the next line.
function layout(page: Page): Promise<Layout> {
  return page.evaluate(() => {
    const chips = [...document.querySelectorAll<HTMLElement>('.vp-doc :not(pre) > code')]
    const wider = document.documentElement.scrollWidth > innerWidth
    return {
      overflow : document.documentElement.scrollWidth - innerWidth,
      split    : chips
        .filter(chip => chip.getClientRects().length > 1 || (wider && chip.getBoundingClientRect().right > innerWidth))
        .map(chip => chip.textContent),
      stranded : chips
        .filter(chip => {
          let at : Node = chip
          while (!at.nextSibling && at.parentElement?.matches('a, em, strong')) at = at.parentElement
          const text = at.nextSibling
          if (chip.getClientRects().length === 0 || !(text instanceof Text) || !/^[,.;:!?)\]}'"’”…]/.test(text.data)) return false
          const range = document.createRange()
          range.setStart(text, 0)
          range.setEnd(text, 1)
          return range.getBoundingClientRect().top >= chip.getBoundingClientRect().bottom - 1
        })
        .map(chip => chip.textContent)
    }
  })
}

// Opens the page in the context and returns what `measure` reads from it.
async function open<T>(context: BrowserContext, file: string, measure: (page: Page) => Promise<T>): Promise<T> {
  const page = await context.newPage()
  try {
    await page.goto(`/${file}`, { waitUntil: 'networkidle' })
    return await measure(page)
  }
  finally {
    await page.close()
  }
}

// Forces every code block, chip, and table to overflow, then reads each rendered
// element a style lets scroll that has no tab stop of its own or inside it, beside
// the regions `axe-core` finds no keyboard reaches.
async function scrolling(page: Page): Promise<Scrolling> {
  await page.addStyleTag({ content: FORCED_OVERFLOW })
  const unreachable = await page.evaluate(focusable => [...document.body.querySelectorAll<HTMLElement>('*')]
    .filter(element => {
      const { overflowX, overflowY } = getComputedStyle(element)
      return [overflowX, overflowY].some(overflow => overflow === 'auto' || overflow === 'scroll')
        && element.getClientRects().length > 0
        && !element.matches(focusable)
        && element.querySelector(focusable) === null
    })
    .map(element => `${element.tagName.toLowerCase()}.${element.classList.value}`), FOCUSABLE)
  return { regions: await audit(page, ['scrollable-region-focusable']), unreachable }
}

let browser  : Browser
let contexts : Record<'narrow' | 'overflowing' | Scheme, BrowserContext>

beforeAll(async () => {
  browser  = await chromium.launch()
  const [dark, light, narrow, overflowing] = await Promise.all([
    bundleContext(browser, { colorScheme: 'dark', viewport: WIDE }),
    bundleContext(browser, { colorScheme: 'light', viewport: WIDE }),
    bundleContext(browser, { viewport: NARROW }),
    bundleContext(browser, { viewport: OVERFLOWING })
  ])
  contexts = { dark, light, narrow, overflowing }
})

afterAll(() => browser.close())

const surveys = new Map<string, Promise<Survey>>()

// Loads the page once in each scheme and at each width, the first test to read it
// starting every load, and holds what each load read for every test after it.
const survey = (file: string): Promise<Survey> => surveys.getOrInsertComputed(file, async () => {
  const [light, dark, narrow, overflow] = await Promise.all([
    open(contexts.light, file, async page => ({ axe: await audit(page), layout: await layout(page) })),
    open(contexts.dark, file, audit),
    open(contexts.narrow, file, layout),
    open(contexts.overflowing, file, scrolling)
  ])
  return { axe: { dark, light: light.axe }, layout: { [NARROW.width]: narrow, [WIDE.width]: light.layout }, overflow }
})

it('finds the built pages', () => {
  expect(BUILT, 'the sweep reads the bundle `mise run site:press` builds').toContain('reference/kit.html')
  expect(PAGES, 'the shard reads a share of the built pages').not.toHaveLength(0)
})

describe.each(SCHEMES)('the built pages in the %s scheme', scheme => {
  it.concurrent.each(PAGES)('%s has no axe violations', async file => {
    expect((await survey(file)).axe[scheme]).toStrictEqual([])
  })
})

describe.each([WIDE.width, NARROW.width])('the built pages at %d px', width => {
  it.concurrent.each(PAGES)('%s holds each inline code chip in one box inside the page', async file => {
    expect((await survey(file)).layout[width]!.split).toStrictEqual([])
  })

  it.concurrent.each(PAGES)('%s fits the viewport\'s width', async file => {
    expect((await survey(file)).layout[width]!.overflow).toBeLessThanOrEqual(0)
  })

  it.concurrent.each(PAGES)('%s holds the punctuation after a chip on its line', async file => {
    expect((await survey(file)).layout[width]!.stranded).toStrictEqual([])
  })
})

describe('the scrolling regions at a forced overflow', () => {
  it.concurrent.each(PAGES)('%s gives each a tab stop', async file => {
    const { regions, unreachable } = (await survey(file)).overflow
    expect(unreachable).toStrictEqual([])
    expect(regions).toStrictEqual([])
  })
})
