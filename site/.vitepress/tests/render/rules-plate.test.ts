import type { Page } from 'playwright'

import { ENGINES, renderTest } from './page'
import type { Engine }         from './page'

interface Frame {
  at     : number
  height : number
  lists  : string
  next   : number
}

const FAMILY = 'alignment'

const NAMES = Object.keys(ENGINES) as Engine[]

const MOVES = NAMES.flatMap(engine => [1280, 390].map(width => ({ engine, width })))

// Sets how far a step may run past the one the curve allows, in pixels, for
// the rounding of a box's edges.
const SLACK = 1

// Sets the widths either side of where each letter's list switches to two
// columns, in each engine.
const SWITCH = NAMES.flatMap(engine => [470, 481, 486, 500, 640, 641].map(width => ({ engine, width })))

// Presses the chip of `family` and reads the plate's height and the top of the
// section below it on each frame from the press until `ms` past it.
const press = (page: Page, family: string, ms: number): Promise<Frame[]> =>
  page.evaluate(([name, until]) => new Promise<Frame[]>(resolve => {
    const chip   = document.querySelector<HTMLElement>(`.rules-plate-split[data-family="${name}"] button`)!
    const plate  = chip.closest('.rules-plate-key')!.parentElement!
    const next   = document.querySelector('#subsetting')!
    const read   = (at: number) => ({
      at,
      height : plate.getBoundingClientRect().height,
      lists  : [...plate.querySelectorAll<HTMLElement>('.rules-plate-letter:not(.is-out):not(.is-ghost) .rules-plate-slugs')]
        .map(list => list.offsetHeight).join(','),
      next   : next.getBoundingClientRect().top + scrollY
    })
    const frames = [read(performance.now())]
    const tick   = (at: number) => {
      frames.push(read(at))
      if (at - frames[0]!.at < until) requestAnimationFrame(tick)
      else resolve(frames)
    }
    chip.click()
    requestAnimationFrame(tick)
  }), [family, ms] as const)

// Returns the steepest slope of the `cubic-bezier()` curve `easing` names.
function steepest(easing: string): number {
  const [x1, y1, x2, y2] = easing.match(/-?[\d.]+/g)!.map(Number) as [number, number, number, number]
  const rate = (one: number, two: number, s: number) =>
    3 * one * (1 - s) ** 2 + 6 * (two - one) * s * (1 - s) + 3 * (1 - two) * s ** 2
  return Math.max(...Array.from({ length: 1001 }, (_, at) => at / 1000)
    .filter(s => rate(x1, x2, s) > 1e-3)
    .map(s => rate(y1, y2, s) / rate(x1, x2, s)))
}

// Lists each frame whose step in `key` from the frame before runs past what a
// move of the whole span over `ms` on a curve no steeper than `slope` allows.
function strays(frames: Frame[], key: 'height' | 'next', slope: number, ms: number): string[] {
  const span = Math.abs(frames.at(-1)![key] - frames[0]![key])
  return frames.slice(1).flatMap((frame, at) => {
    const before = frames[at]!
    const step   = Math.abs(frame[key] - before[key])
    const room   = slope * span * Math.max(frame.at - before.at, 0) / ms + SLACK
    const when   = Math.round(frame.at - frames[0]!.at)
    return step > room ? [`${key} steps ${step.toFixed(1)}px at ${when}ms against ${room.toFixed(1)}px`] : []
  })
}

// Scrolls the rules index to its key and reads the curve and duration each
// move takes.
async function rulesIndex(page: Page): Promise<{ ms: number, slope: number }> {
  await page.locator('.rules-plate-key').scrollIntoViewIfNeeded()
  const [easing, ms] = await page.evaluate(() => {
    const root = getComputedStyle(document.documentElement)
    return [root.getPropertyValue('--prose-ease-morph'), Number(root.getPropertyValue('--prose-rule-draw-ms'))] as const
  })
  return { ms, slope: steepest(easing) }
}

describe('the rules index', () => {
  renderTest.for(MOVES)('eases its height and the section below with the rows, each letter holding its layout, in $engine at $width px', async ({ engine, width }, { visit }) => {
    const page          = await visit('/rules/', { engine, width })
    const { ms, slope } = await rulesIndex(page)

    const moves = [] as { change: number, layouts: number, strays: string[] }[]
    for (let at = 0; at < 2; at++) {
      const frames = await press(page, FAMILY, ms + 300)
      moves.push({
        change  : Math.sign(frames.at(-1)!.height - frames[0]!.height),
        layouts : new Set(frames.slice(1).map(({ lists }) => lists)).size,
        strays  : [...strays(frames, 'height', slope, ms), ...strays(frames, 'next', slope, ms)]
      })
    }

    expect(moves).toStrictEqual([
      { change: moves[0]!.change, layouts: 1, strays: [] },
      { change: -moves[0]!.change, layouts: 1, strays: [] }
    ])
    expect(moves[0]!.change).not.toBe(0)
  })

  renderTest.for(MOVES)('starts a press during a move from the height on screen in $engine at $width px', async ({ engine, width }, { visit }) => {
    const page          = await visit('/rules/', { engine, width })
    const { ms, slope } = await rulesIndex(page)

    await press(page, FAMILY, ms / 10)
    const frames = await press(page, FAMILY, ms + 300)

    expect([...strays(frames, 'height', slope, ms), ...strays(frames, 'next', slope, ms)]).toStrictEqual([])
  })

  renderTest.for(SWITCH)('keeps each slug on one line in $engine at $width px', async ({ engine, width }, { visit }) => {
    const page = await visit('/rules/', { engine, width })

    const wrapped = await page.locator('.rules-plate-letter:not(.is-out) .rules-plate-slug').evaluateAll(slugs =>
      slugs.filter(slug => {
        const text = document.createRange()
        text.selectNodeContents(slug)
        return text.getClientRects().length > 1
      }).map(slug => slug.textContent))

    expect(wrapped).toStrictEqual([])
  })

  renderTest.for(NAMES)('takes its new height in the first frame under reduced motion in %s', async (engine, { visit }) => {
    const page   = await visit('/rules/', { engine, motion: 'reduce', width: 390 })
    const { ms } = await rulesIndex(page)

    const frames = await press(page, FAMILY, ms)

    expect(frames[1]!.height).not.toBe(frames[0]!.height)
    expect(frames[1]!.height).toBe(frames.at(-1)!.height)
  })
})
