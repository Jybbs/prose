import axe                                      from 'axe-core'
import { clampRgb, converter, wcagContrast } from 'culori'
import { JSDOM }                                from 'jsdom'
import type { Locator, Page }                   from 'playwright'

import { LEVELS } from '../lib/identity/roles'

export interface Finding {
  id      : string
  targets : string[]
}

interface Ground {
  ink     : string
  opacity : number
  pixels  : [string, string][]
}

type LineBox = Pick<DOMRectReadOnly, 'height' | 'width' | 'x' | 'y'>

interface Reach {
  runOnly? : string[]
  scope?   : string
}

export const AXE_TIMEOUT_MS = 15_000
const BARE                  = 'data-ground-bare'
const GROUND                = 'data-ground'
const UNDER                 = 'data-ground-under'
// The `incomplete` reasons `axe-core` 4.14 gives for a lone character or a glyph
// outside the Basic Multilingual Plane, which reads as decoration where
// `aria-hidden` hides it.
const GLYPH_REASONS = ['nonBmp', 'shortTextContent']
// The `incomplete` reasons `axe-core` 4.14 gives for a contrast it could not
// resolve on text.
const GROUND_REASONS = [
  'bgGradient', 'bgImage', 'bgOverlap', 'elmPartiallyObscured', 'elmPartiallyObscuring', 'fgAlpha',
  'imgNode', 'outsideViewport', 'pseudoContent', ...GLYPH_REASONS
]
const rgb            = converter('rgb')
// SC 1.4.3 Contrast (Minimum), https://www.w3.org/TR/WCAG22/#contrast-minimum
const textFloor      = Object.fromEntries(LEVELS).text
const CAPTURE_STYLES = `[${BARE}], [${BARE}] * {
  color                   : transparent !important;
  fill                    : transparent !important;
  stroke                  : transparent !important;
  -webkit-text-fill-color : transparent !important;
  text-decoration-color   : transparent !important;
  text-shadow             : none !important;
  transition              : none !important;
}
[${UNDER}="opaque"] {
  opacity    : 1 !important;
  transition : none !important;
}`

let shared: JSDOM | undefined

// Builds the test file's one JSDOM on the first call, with the axe source
// evaluated into it, and gives every later check the same one, each check
// replacing the body the one before it rendered.
function axeDom(): JSDOM {
  if (!shared) {
    shared = new JSDOM('<!DOCTYPE html><body></body>', { runScripts: 'outside-only' })
    shared.window.eval(axe.source)
  }
  return shared
}

// Reads the text color of the element, the opacity it and its ancestors
// multiply to, and each pair of colors the page renders under its ink above the
// baseline, where a descender crossing an underline cannot reach. The element
// is captured once as drawn and once with its text transparent, so a pixel the
// two differ on is one the text covers, and once more with its text still
// transparent and every faded element from it up drawn opaque, giving the color
// the text covers with every fade lifted at each pixel.
async function groundOf(element: Locator): Promise<Ground> {
  const { ink, lines, opacity } = await element.evaluate((node, marker) => {
    const style   = getComputedStyle(node)
    const frame   = node.getBoundingClientRect()
    const range   = document.createRange()
    const measure = new OffscreenCanvas(1, 1).getContext('2d')!
    const texts   = [...node.childNodes]
      .filter(child => child.nodeType === Node.TEXT_NODE && child.textContent?.trim())
    measure.font = style.font
    const { fontBoundingBoxAscent: ascent, fontBoundingBoxDescent: descent } = measure
      .measureText(texts[0]?.textContent ?? '')
    const above = ascent / (ascent + descent)
    let alpha   = 1
    for (let at: Element | null = node; at; at = at.parentElement) {
      const own = Number(getComputedStyle(at).opacity)
      alpha *= own
      if (own < 1) at.setAttribute(marker, '')
    }
    return {
      ink     : node instanceof SVGElement ? style.fill : style.webkitTextFillColor,
      opacity : alpha,
      lines   : texts.flatMap(text => {
        range.selectNodeContents(text)
        return [...range.getClientRects()].map(({ height, width, x, y }) =>
          ({ height: height * above, width, x: x - frame.x, y: y - frame.y }))
      })
    }
  }, UNDER)
  const captured = async (): Promise<string> =>
    (await element.screenshot({ animations: 'disabled', scale: 'css' })).toString('base64')
  const shown = await captured()
  await element.evaluate((node, marker) => { node.setAttribute(marker, '') }, BARE)
  const bare = await captured()
  await element.page().evaluate(marker => {
    document.querySelectorAll(`[${marker}]`).forEach(faded => { faded.setAttribute(marker, 'opaque') })
  }, UNDER)
  const under = opacity < 1 ? await captured() : bare
  await element.evaluate((node, { group, text }) => {
    node.removeAttribute(text)
    document.querySelectorAll(`[${group}]`).forEach(faded => { faded.removeAttribute(group) })
  }, { group: UNDER, text: BARE })
  const pixels = await element.page().evaluate(underInk, { bare, lines, shown, under })
  return { ink, opacity, pixels }
}

// Moves the ground the page renders at each pixel toward the text color by the
// text color's alpha times the opacity, measured from the color the text covers
// with every fade lifted, and returns the lowest ratio any pixel holds, or zero
// for a text that changed no pixel.
function lowestContrast({ ink, opacity, pixels }: Ground): number {
  const text = rgb(ink)
  if (!text || pixels.length === 0) return 0
  const weight = (text.alpha ?? 1) * opacity
  return Math.min(...pixels.map(([bare, under]) => {
    const ground  = rgb(bare)!
    const opaque  = rgb(under)!
    const shifted = clampRgb({
      b    : ground.b + weight * (text.b - opaque.b),
      g    : ground.g + weight * (text.g - opaque.g),
      mode : 'rgb',
      r    : ground.r + weight * (text.r - opaque.r)
    })
    return wcagContrast(shifted, ground)
  }))
}

// Decodes the captures in the page and returns each distinct pair of colors the
// bare and the opaque-group captures hold at a pixel the drawn capture differs
// on, inside the boxes the element's own text spans.
async function underInk(
  { bare, lines, shown, under } : { bare: string, lines: LineBox[], shown: string, under: string }
): Promise<[string, string][]> {
  const color  = (image: ImageData, at: number): string => `rgb(${image.data.subarray(at, at + 3).join(' ')})`
  const decode = async (png: string) => {
    const bitmap  = await createImageBitmap(new Blob([Uint8Array.fromBase64(png)]))
    const context = new OffscreenCanvas(bitmap.width, bitmap.height).getContext('2d')!
    context.drawImage(bitmap, 0, 0)
    return context.getImageData(0, 0, bitmap.width, bitmap.height)
  }
  const [ground, drawn, beneath] = await Promise.all([decode(bare), decode(shown), decode(under)])
  const colors = new Map<string, [string, string]>()
  for (const { height, width, x, y } of lines) {
    const bottom = Math.min(ground.height, Math.ceil(y + height))
    const right  = Math.min(ground.width, Math.ceil(x + width))
    for (let row = Math.max(0, Math.floor(y)); row < bottom; row++) {
      for (let column = Math.max(0, Math.floor(x)); column < right; column++) {
        const at    = (row * ground.width + column) * 4
        const inked = ground.data.subarray(at, at + 3).some((channel, index) => channel !== drawn.data[at + index])
        if (!inked) continue
        const pair: [string, string] = [color(ground, at), color(beneath, at)]
        colors.set(pair.join(), pair)
      }
    }
  }
  return [...colors.values()]
}

// Runs `axe-core` over the page, or over the elements the Playwright selector
// `scope` matches, limited to the rules `runOnly` names where it names any. A
// text whose contrast `axe-core` leaves unresolved, a glyph `aria-hidden` hides
// aside, fails unless every color the page renders under its ink holds the text
// floor `LEVELS` declares against it, the failure naming the lowest ratio.
export async function audit(page: Page, { runOnly, scope = 'html' }: Reach = {}): Promise<Finding[]> {
  await page.addScriptTag({ content: axe.source })
  const options = {
    glyphs  : GLYPH_REASONS,
    marker  : GROUND,
    only    : runOnly ? { runOnly } : {},
    reasons : GROUND_REASONS
  }
  const { findings, unresolved } = await page.locator(scope).evaluateAll(async (nodes, settings) => {
    const engine  = (window as unknown as { axe: typeof axe }).axe
    const run     = await engine.run(nodes, { ...settings.only, elementRef: true })
    const targets = (results: axe.NodeResult[]) => results.map(node => node.target.join(' '))
    const decoration = (node: axe.NodeResult, reason: string) =>
      settings.glyphs.includes(reason) && Boolean(node.element?.closest('[aria-hidden="true"]'))
    const grounds = (run.incomplete.find(result => result.id === 'color-contrast')?.nodes ?? [])
      .filter(node => node.any.some(({ data }) =>
        settings.reasons.includes(data?.messageKey) && !decoration(node, data.messageKey)))
    grounds.forEach((node, at) => { node.element?.setAttribute(settings.marker, String(at)) })
    return {
      findings   : run.violations.map(result => ({ id: result.id, targets: targets(result.nodes) })),
      unresolved : targets(grounds)
    }
  }, options)
  if (unresolved.length > 0) await page.addStyleTag({ content: CAPTURE_STYLES })
  const failed: string[] = []
  for (const [at, target] of unresolved.entries()) {
    const ratio = lowestContrast(await groundOf(page.locator(`[${GROUND}="${at}"]`)))
    if (ratio < textFloor) failed.push(`${target} at ${ratio.toFixed(2)}:1`)
  }
  return failed.length > 0 ? [...findings, { id: 'color-contrast', targets: failed }] : findings
}

export async function expectAccessible(
  html   : string,
  ignore : readonly string[] = []
): Promise<void> {
  const { window } = axeDom()
  window.document.body.innerHTML = html
  const run = await (window as unknown as { axe: typeof axe }).axe.run(window.document.body, {
    rules: {
      'color-contrast' : { enabled: false },
      region           : { enabled: false }
    }
  })
  // Copies the violations out of the JSDOM realm axe runs in, whose array
  // prototype differs from this realm's and fails a strict compare.
  expect([...run.violations].filter(v => !ignore.includes(v.id))).toStrictEqual([])
}

export function rendersAccessibly(html: () => string): void {
  it('renders with no axe violations', async () => {
    await expectAccessible(html())
  }, AXE_TIMEOUT_MS)
}
