import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import { type Browser, chromium, type Page } from 'playwright'
import { preview }                           from 'vite'

import { LoopEncoder }          from './encode'
import * as shareLink           from '../sandbox/share-link'
import { type Scheme, SCHEMES } from '../shared/palette'
import { readmeDir, siteDir }   from '../shared/paths'

// Holds the frame the GIF steps by and the frame counts each phase takes, the
// lead before the first rule, the hover over a tag before its click, the least
// a rule's change runs, the frames with no live animation that end it, the
// linger after it, and the hold on the result, beside the viewport's width.
const RECORDING = {
  frameMs : 20,
  hold    : 130,
  hover   : 14,
  lead    : 60,
  linger  : 26,
  quiet   : 6,
  settle  : 30,
  width   : 1240
} as const

const CHANGE_POLLS      = 40
const CLOCK_PAUSE       = new Date('2026-01-01T12:05:00Z')
const CLOCK_START       = new Date('2026-01-01T12:00:00Z')
const MARGIN            = 12
const QUIET_POLLS       = 4
const QUIET_POLL_MS     = 25
const SAMPLES           = 14
const SCALE             = 2
const SETTLE_MAX        = 400
const SOURCE            = fs.readFileSync(path.join(import.meta.dirname, 'retry.py'), 'utf8')

// Records the Sandbox on the README source in each scheme, serving the pressed
// bundle under `vitepressDir`, and writes each as `sandbox-<scheme>.gif` under
// `outDir`, keeping the frames in a scratch directory it removes once the GIF
// is written.
async function recordSandbox(outDir: string, vitepressDir: string): Promise<void> {
  const server  = await preview({ appType: 'mpa', configFile: false, logLevel: 'silent', root: vitepressDir })
  const browser = await chromium.launch()
  try {
    const hash    = shareLink.HASH_PREFIX + await shareLink.encodeShare({ configToml: '', source: SOURCE })
    const url     = `${server.resolvedUrls!.local[0]!}sandbox/${hash}`
    const decoder = await browser.newPage()
    fs.mkdirSync(outDir, { recursive: true })
    for (const scheme of SCHEMES) {
      using scratch = fs.mkdtempDisposableSync(path.join(os.tmpdir(), 'prose-readme-'))
      await recordScheme(browser, scratch.path, scheme, url)
      fs.writeFileSync(path.join(outDir, `sandbox-${scheme}.gif`), await encodeFrames(decoder, scratch.path))
    }
  }
  finally {
    await browser.close()
    await server.close()
  }
}

// Halves a PNG frame in the browser and returns its RGBA pixels with its size.
function decode(page: Page, png: Buffer): Promise<{ height: number, rgba: Uint8Array, width: number }> {
  return page.evaluate(async ({ bytes, scale }) => {
    const source = await createImageBitmap(new Blob([bytes]))
    const width  = source.width / scale
    const height = source.height / scale
    const bitmap = await createImageBitmap(source, { resizeHeight: height, resizeQuality: 'high', resizeWidth: width })
    const canvas = new OffscreenCanvas(width, height).getContext('2d')!
    canvas.drawImage(bitmap, 0, 0)
    return { height, rgba: new Uint8Array(canvas.getImageData(0, 0, width, height).data.buffer), width }
  }, { bytes: new Uint8Array(png), scale: SCALE })
}

// Encodes the numbered frames under `frameDir` into one GIF against a palette
// quantized from `SAMPLES` frames spread across the loop.
async function encodeFrames(decoder: Page, frameDir: string): Promise<Uint8Array> {
  const frames  = fs.readdirSync(frameDir).sort().map(name => path.join(frameDir, name))
  const read    = (file: string): ReturnType<typeof decode> => decode(decoder, fs.readFileSync(file))
  const samples = await Promise.all(Array.from({ length: SAMPLES },
    (_, at) => read(frames[Math.floor(at * (frames.length - 1) / (SAMPLES - 1))]!)))
  const { height, width } = samples[0]!
  const encoder = new LoopEncoder(RECORDING.frameMs, height, width, samples.map(sample => sample.rgba))
  for (const file of frames) encoder.add((await read(file)).rgba)
  return encoder.bytes()
}

async function recordScheme(
  browser : Browser,
  outDir  : string,
  scheme  : Scheme,
  url     : string
): Promise<void> {
  const context = await browser.newContext({
    colorScheme       : scheme,
    deviceScaleFactor : SCALE,
    reducedMotion     : 'no-preference',
    viewport          : { height: 1600, width: RECORDING.width }
  })
  const page = await context.newPage()
  await page.clock.install({ time: CLOCK_START })
  const cdp = await context.newCDPSession(page)
  await cdp.send('Animation.enable')
  await page.goto(url)

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
    height : Math.ceil(Math.max(onBottom, await panelBottom(page)) - top + MARGIN),
    width  : deck.width + 2 * MARGIN,
    x      : Math.max(0, deck.x - MARGIN),
    y      : top
  }

  let frame = 0
  const tick = async (polls = QUIET_POLLS): Promise<number> => {
    const live = await page.evaluate(ms => window.stepAnimations!(ms), RECORDING.frameMs)
    await page.clock.runFor(RECORDING.frameMs)
    await quiesce(page, polls)
    await page.screenshot({ clip, path: path.join(outDir, `f${String(frame++).padStart(5, '0')}.png`) })
    return live
  }
  const ticks = async (count: number): Promise<void> => {
    for (let i = 0; i < count; i++) await tick()
  }

  const tags = await page.$$eval('.sandbox-tags-tag', nodes => nodes.map(tag => tag.querySelector('button')!.textContent!.trim()))
  await ticks(RECORDING.lead)
  for (const slug of tags) {
    const box = (await page.locator('.sandbox-tags-tag', { hasText: slug }).locator('button').first().boundingBox())!
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await ticks(RECORDING.hover)
    const before = await panelMutations(page)
    await page.mouse.down()
    await page.mouse.up()
    let quiet = 0
    for (let since = 1; since <= RECORDING.settle || quiet < RECORDING.quiet; since++) {
      if (since > SETTLE_MAX) throw new Error(`the change ${slug} makes never settles`)
      const polls = await panelMutations(page) === before ? CHANGE_POLLS : QUIET_POLLS
      quiet = await tick(polls) === 0 ? quiet + 1 : 0
    }
    await ticks(RECORDING.linger)
    await expectState(page, `the frame after ${slug}`, { allOff: false })
  }
  await ticks(RECORDING.hold)
  await context.close()
}

declare global {
  interface Window {
    panelMutations?    : number
    recordedMutations? : number
    stepAnimations?    : (ms: number) => number
  }
}

// Holds every animation the page starts at its first frame, then advances each
// by `ms` per call, returning how many are still running, and counts every DOM
// mutation in `recordedMutations` and every one inside the code panel in
// `panelMutations`.
function installStepper(): void {
  const seen    = new WeakSet<Animation>()
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
      animation.currentTime = seen.has(animation) ? Number(animation.currentTime ?? 0) + ms : 0
      seen.add(animation)
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
