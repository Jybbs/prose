import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import { type Browser, chromium, type Page } from 'playwright'
import { preview }                           from 'vite'

import { LoopEncoder }          from './encode'
import * as rulerScale          from '../sandbox/ruler-scale'
import * as shareLink           from '../sandbox/share-link'
import { type Scheme, SCHEMES } from '../shared/palette'
import { readmeDir, siteDir }   from '../shared/paths'

type Tick = (polls?: number, capture?: boolean) => Promise<number>

// Holds the frame the GIF steps by and the milliseconds each phase takes, the
// lead before the first rule, the hover over a control before its press, the
// least a change runs, the linger after it, and the hold on the result, beside
// the frames with no live animation that end a change, the line length the
// ruler drags to and the columns it crosses each second, the stride the rewind
// takes through the frames that moved, the GIF's width in pixels, and the
// viewport's width.
const RECORDING = {
  dragRate : 100,
  dragTo   : 50,
  frameMs  : 40,
  hold     : 500,
  hover    : 80,
  lead     : 200,
  linger   : 400,
  out      : 1756,
  quiet    : 6,
  rewind   : 5,
  settle   : 480,
  width    : 1360
} as const

const CHANGE_POLLS      = 40
const CLOCK_PAUSE       = new Date('2026-01-01T12:05:00Z')
const CLOCK_START       = new Date('2026-01-01T12:00:00Z')
const MARGIN            = 12
const PRECOMPILE_MS     = 2500
const QUIET_POLLS       = 4
const QUIET_POLL_MS     = 25
const SAMPLES           = 14
const SETTLE_MAX        = 400
const SOURCE            = fs.readFileSync(path.join(import.meta.dirname, 'retry.py'), 'utf8')
// Draws the Sandbox's editor and code panel alone, one column wide, with the
// rule bar and ruler enlarged to read at the README's width.
const STAGE             = `
.w-sandbox > div.grid > :nth-child(2), .sandbox-tags-banner { display: none !important }
.w-sandbox > div.grid { grid-template-columns: 1fr !important }
.w-sandbox > div.grid > * { min-height: 0 !important }
.w-sandbox > section { zoom: 1.45 }
`
const VIEWPORT_HEIGHT   = 2400
const CLIP_WIDTH        = RECORDING.width - 2 * MARGIN
const SCALE             = RECORDING.out / CLIP_WIDTH

// Records the Sandbox on the README source in each scheme, serving the pressed
// bundle under `vitepressDir`, and writes each as `sandbox-<scheme>.gif` under
// `outDir`, keeping the frames in a scratch directory it removes once the GIF
// is written.
async function recordSandbox(outDir: string, vitepressDir: string): Promise<void> {
  const server  = await preview({ appType: 'mpa', configFile: false, logLevel: 'silent', root: vitepressDir })
  const browser = await chromium.launch({ args: ['--mute-audio'] })
  try {
    const hash    = shareLink.HASH_PREFIX + await shareLink.encodeShare({ configToml: '', source: SOURCE })
    const url     = `${server.resolvedUrls!.local[0]!}sandbox/${hash}`
    const decoder = await browser.newPage()
    fs.mkdirSync(outDir, { recursive: true })
    for (const scheme of SCHEMES) {
      using scratch = fs.mkdtempDisposableSync(path.join(os.tmpdir(), 'prose-readme-'))
      const rows    = await recordScheme(browser, scratch.path, scheme, url)
      fs.writeFileSync(path.join(outDir, `sandbox-${scheme}.gif`), await encodeFrames(decoder, scratch.path, rows))
    }
  }
  finally {
    await browser.close()
    await server.close()
  }
}

// Crops a PNG frame to its top `rows` page pixels in the browser and returns
// its RGBA pixels with its size.
function decode(
  page : Page,
  png  : Buffer,
  rows : number
): Promise<{ height: number, rgba: Uint8Array, width: number }> {
  return page.evaluate(async ({ bytes, kept, scale }) => {
    const source = await createImageBitmap(new Blob([bytes]))
    const width  = source.width
    const height = Math.min(source.height, Math.round(kept * scale))
    const canvas = new OffscreenCanvas(width, height).getContext('2d')!
    canvas.drawImage(source, 0, 0)
    return { height, rgba: new Uint8Array(canvas.getImageData(0, 0, width, height).data.buffer), width }
  }, { bytes: new Uint8Array(png), kept: rows, scale: SCALE })
}

// Encodes the numbered frames under `frameDir` into one GIF against a palette
// quantized from `SAMPLES` frames spread across the loop, dropping a frame held
// alone between two that move, then rewinds to the opening frame through every
// `RECORDING.rewind`th frame that moved, taken in reverse, so the loop returns
// to its start rather than cutting to it.
async function encodeFrames(decoder: Page, frameDir: string, rows: number): Promise<Uint8Array> {
  const frames  = fs.readdirSync(frameDir).sort().map(name => path.join(frameDir, name))
  const pixels  = (file: string): ReturnType<typeof decode> => decode(decoder, fs.readFileSync(file), rows)
  const read    = async (file: string): Promise<Buffer> => Buffer.from((await pixels(file)).rgba)
  const samples = await Promise.all(Array.from({ length: SAMPLES },
    (_, at) => pixels(frames[Math.floor(at * (frames.length - 1) / (SAMPLES - 1))]!)))
  const { height, width } = samples[0]!
  const encoder = new LoopEncoder(RECORDING.frameMs, height, width, samples.map(sample => sample.rgba))
  const moved: string[] = []
  let current     = await read(frames[0]!)
  let movedBefore = false
  let previous    = current
  for (const [at, file] of frames.entries()) {
    const next      = at + 1 < frames.length ? await read(frames[at + 1]!) : null
    const moves     = at > 0 && !previous.equals(current)
    const movesNext = next !== null && !current.equals(next)
    if (moves) moved.push(file)
    if (moves || !movedBefore || !movesNext) encoder.add(current)
    movedBefore = moves
    previous    = current
    current     = next ?? current
  }
  const rewind = moved.slice(1).toReversed().filter((_, at) => at % RECORDING.rewind === 0)
  for (const file of [...rewind, frames[0]!]) encoder.add(await read(file))
  return encoder.bytes()
}

// Drags the ruler's marker to `RECORDING.dragTo` at `RECORDING.dragRate`
// columns a second, then ticks until the code panel's reflow starts and
// settles.
async function dragRuler(page: Page, tick: Tick, ticks: (ms: number) => Promise<void>): Promise<void> {
  const geometry = await page.evaluate(() => {
    const track  = document.querySelector('.sandbox-ruler-track')!.getBoundingClientRect()
    const marker = document.querySelector('.sandbox-ruler-stop .sandbox-ruler-marker')!.getBoundingClientRect()
    return {
      left    : track.left,
      markerX : marker.x + marker.width / 2,
      markerY : marker.y + marker.height / 2,
      width   : track.width
    }
  })
  const span    = rulerScale.LENGTH_MAX - rulerScale.LENGTH_MIN
  const target  = geometry.left + (RECORDING.dragTo - rulerScale.LENGTH_MIN) / span * geometry.width
  const columns = Math.abs(target - geometry.markerX) / geometry.width * span
  const steps   = Math.round(columns / (RECORDING.dragRate * RECORDING.frameMs / 1000))
  await page.mouse.move(geometry.markerX, geometry.markerY)
  await ticks(RECORDING.hover)
  await page.waitForTimeout(PRECOMPILE_MS)
  await page.mouse.down()
  for (let step = 1; step <= steps; step++) {
    await page.mouse.move(geometry.markerX + (target - geometry.markerX) * step / steps, geometry.markerY)
    await tick(CHANGE_POLLS)
  }
  await ticks(2 * RECORDING.frameMs)
  await page.mouse.up()
  const moving = (): Promise<boolean> => page.evaluate(() => document.querySelector('.code-panel')!
    .getAnimations({ subtree: true })
    .some(animation => animation.playState !== 'finished'))
  for (let since = 1; !await moving(); since++) {
    if (since > SETTLE_MAX) throw new Error('the drag never moves the code')
    await tick(CHANGE_POLLS, false)
  }
  let quiet = 0
  for (let since = 1; since * RECORDING.frameMs <= RECORDING.settle || quiet < RECORDING.quiet; since++) {
    if (since > SETTLE_MAX) throw new Error('the drag never settles')
    quiet = await tick(CHANGE_POLLS) === 0 ? quiet + 1 : 0
  }
  await ticks(RECORDING.linger)
}

// Records one scheme's frames under `outDir` and returns the page pixels from
// the clip's top to the lowest the code panel reached, the rows each frame
// keeps.
async function recordScheme(
  browser : Browser,
  outDir  : string,
  scheme  : Scheme,
  url     : string
): Promise<number> {
  const context = await browser.newContext({
    colorScheme       : scheme,
    deviceScaleFactor : SCALE,
    reducedMotion     : 'no-preference',
    viewport          : { height: VIEWPORT_HEIGHT, width: RECORDING.width }
  })
  const page = await context.newPage()
  await page.clock.install({ time: CLOCK_START })
  const cdp = await context.newCDPSession(page)
  await cdp.send('Animation.enable')
  await page.goto(url)
  await page.addStyleTag({ content: STAGE })

  await until(page, () => document.querySelectorAll('.sandbox-tags-tag').length > 0, 'the rule tags')
  await until(page, () => document.querySelector('.code-panel pre') !== null, 'the code panel')
  await idle(page, 60)
  const onBottom = await panelBottom(page)
  await page.getByRole('button', { name: 'Turn every rule off' }).click()
  await idle(page, 80)
  await page.mouse.move(2, 2)
  await idle(page, 40)
  await expectState(page, 'the opening frame', { allOff: true })

  await page.clock.pauseAt(CLOCK_PAUSE)
  await page.evaluate(installStepper)
  await cdp.send('Animation.setPlaybackRate', { playbackRate: 0 })

  const deck  = await page.evaluate(() => {
    const root = document.querySelector('.w-sandbox')!.getBoundingClientRect()
    const top  = document.querySelector('.w-sandbox > section')!.getBoundingClientRect()
    return { top: top.y, width: root.width, x: root.x }
  })
  const top  = deck.top - MARGIN
  const clip = {
    height : VIEWPORT_HEIGHT - top,
    width  : deck.width + 2 * MARGIN,
    x      : Math.max(0, deck.x - MARGIN),
    y      : top
  }
  if (Math.abs(clip.width - CLIP_WIDTH) > 1) throw new Error(`the clip is ${clip.width} px wide, not ${CLIP_WIDTH}`)

  let frame  = 0
  let bottom = Math.max(onBottom, await panelBottom(page))
  const tick: Tick = async (polls = QUIET_POLLS, capture = true) => {
    const live = await page.evaluate(ms => window.stepAnimations!(ms), RECORDING.frameMs)
    await page.clock.runFor(RECORDING.frameMs)
    await quiesce(page, polls)
    bottom = Math.max(bottom, await panelBottom(page))
    if (capture) await page.screenshot({ clip, path: path.join(outDir, `f${String(frame++).padStart(5, '0')}.png`) })
    return live
  }
  const ticks = async (ms: number): Promise<void> => {
    for (let i = 0; i < Math.round(ms / RECORDING.frameMs); i++) await tick()
  }

  const tags = await page.$$eval('.sandbox-tags-tag', nodes => nodes.map(tag => tag.querySelector('button')!.textContent!.trim()))
  await ticks(RECORDING.lead)
  for (const slug of tags) {
    const box = (await page.locator('.sandbox-tags-tag', { hasText: slug }).locator('button').first().boundingBox())!
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await ticks(RECORDING.hover)
    const before = await panelMutations(page)
    await page.waitForTimeout(PRECOMPILE_MS)
    await page.mouse.down()
    await page.mouse.up()
    let quiet = 0
    for (let since = 1; since * RECORDING.frameMs <= RECORDING.settle || quiet < RECORDING.quiet; since++) {
      if (since > SETTLE_MAX) throw new Error(`the change ${slug} makes never settles`)
      const polls = await panelMutations(page) === before ? CHANGE_POLLS : QUIET_POLLS
      quiet = await tick(polls) === 0 ? quiet + 1 : 0
    }
    await ticks(RECORDING.linger)
    await expectState(page, `the frame after ${slug}`, { allOff: false })
  }
  await dragRuler(page, tick, ticks)
  await ticks(RECORDING.hold)
  await context.close()
  return Math.ceil(bottom - top + MARGIN)
}

declare global {
  interface Window {
    panelMutations?    : number
    recordedMutations? : number
    stepAnimations?    : (ms: number) => number
  }
}

// Advances every animation by `ms` per call from where it stands, so one the
// page started since the last call, already captured at its first frame, moves
// on rather than holding that frame twice, returning how many are still
// running, and counts every DOM mutation in `recordedMutations` and every one
// inside the code panel in `panelMutations`.
function installStepper(): void {
  const options = { attributes: true, characterData: true, childList: true, subtree: true }
  window.panelMutations    = 0
  window.recordedMutations = 0
  new MutationObserver(records => { window.recordedMutations! += records.length }).observe(document.body, options)
  new MutationObserver(records => { window.panelMutations! += records.length })
    .observe(document.querySelector('.code-panel')!, options)
  window.stepAnimations = ms => {
    let live = 0
    for (const animation of document.getAnimations()) {
      animation.pause()
      animation.currentTime = Number(animation.currentTime ?? 0) + ms
      if (animation.playState !== 'finished') live++
    }
    return live
  }
}

// Fails the recording where the code panel carries a lint finding, or where
// the rule tags are not all off at the opening frame.
async function expectState(page: Page, label: string, { allOff }: { allOff: boolean }): Promise<void> {
  const { flags, off, tags } = await page.evaluate(() => ({
    flags : document.querySelectorAll('.code-panel .lint-flag').length,
    off   : document.querySelectorAll('.sandbox-tags-tag[data-off]').length,
    tags  : document.querySelectorAll('.sandbox-tags-tag').length
  }))
  if (flags > 0)              throw new Error(`${label} shows ${flags} lint findings`)
  if (allOff && off !== tags) throw new Error(`${label} shows ${tags - off} of ${tags} rules on`)
}

// Waits in real time until the DOM has gone `polls` polls without a mutation,
// so work the page runs outside the paused clock lands before the frame rather
// than at a tick the machine's speed decides. A tick after a click whose change
// has not yet reached the code panel waits `CHANGE_POLLS`, which covers the
// formatter, the highlighter, and the morph's precompile.
async function quiesce(page: Page, polls: number): Promise<void> {
  let count = -1
  for (let quiet = 0; quiet < polls;) {
    await page.waitForTimeout(QUIET_POLL_MS)
    const next = await page.evaluate(() => window.recordedMutations!)
    quiet      = next === count ? quiet + 1 : 0
    count      = next
  }
}

function panelMutations(page: Page): Promise<number> {
  return page.evaluate(() => window.panelMutations!)
}

async function idle(page: Page, frames: number): Promise<void> {
  for (let i = 0; i < frames; i++) {
    await page.clock.runFor(50)
    await page.waitForTimeout(30)
  }
}

function panelBottom(page: Page): Promise<number> {
  return page.evaluate(() => document.querySelector('.code-panel')!.getBoundingClientRect().bottom)
}

async function until(page: Page, test: () => boolean, label: string): Promise<void> {
  for (let i = 0; i < SETTLE_MAX; i++) {
    if (await page.evaluate(test)) return
    await page.clock.runFor(50)
    await page.waitForTimeout(50)
  }
  throw new Error(`timed out waiting for ${label}`)
}

if (import.meta.main) {
  await recordSandbox(readmeDir(import.meta.url), path.join(siteDir(import.meta.url), '.vitepress'))
}
