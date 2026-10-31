import { defaultDocument, useEventListener } from '@vueuse/core'

// Matches each element `nib-stroke.css` draws the link rule and the nib stroke
// on, meaning a link's own words, a code span a link holds, and any element
// carrying the `nib-stroke` class.
export const NIB_MARKS = 'a.body-link:not(:has(code)), :is(a.body-link, a.tool-mark) code, .nib-stroke'

interface Line {
  bottom : number
  left   : number
  top    : number
  width  : number
}

// Returns the mark `node` sits in, or the mark inside the link or button it
// sits in.
function markOf(node: EventTarget | null): HTMLElement | null {
  if (!(node instanceof Element)) return null
  return node.closest<HTMLElement>(NIB_MARKS)
    ?? node.closest('a, button')?.querySelector<HTMLElement>(NIB_MARKS)
    ?? null
}

// Moves the nib stroke under the pointer along the line of a mark it rests
// on. Entering a mark reads its font size and its line boxes once, in page
// coordinates, and each pointer move over it queues one frame, which writes
// the pointer's place along that line as `--nib-at`, kept half a stroke
// inside either end of the line. A keyboard focus clears the place, which
// returns the stroke to the middle of each line.
export function useNibStroke(doc: Document | undefined = defaultDocument): void {
  const win  = doc?.defaultView
  let mark   = null as HTMLElement | null
  let lines  = [] as Line[]
  let half   = 0
  let queued = 0
  let x      = 0
  let y      = 0

  const place = (): void => {
    queued = 0
    const line = mark ? lines.find(each => y >= each.top && y <= each.bottom) : undefined
    if (!mark || !line) return
    const at = Math.min(Math.max(x - line.left, half), Math.max(half, line.width - half))
    mark.style.setProperty('--nib-at', `calc(${Math.round(at)}px - var(--nib-swell) * var(--prose-size-2-5em) / 2)`)
  }

  useEventListener(doc, 'pointerover', (event: PointerEvent) => {
    const next = markOf(event.target)
    x = event.pageX
    y = event.pageY
    if (next === mark) return
    mark = next
    if (!mark || !win) return
    half  = parseFloat(win.getComputedStyle(mark).fontSize) * 1.25
    lines = [...mark.getClientRects()].map(rect => ({
      bottom : rect.bottom + win.scrollY,
      left   : rect.left + win.scrollX,
      top    : rect.top + win.scrollY,
      width  : rect.width
    }))
    place()
  }, { passive: true })

  useEventListener(doc, 'pointermove', (event: PointerEvent) => {
    if (!mark || !win) return
    x = event.pageX
    y = event.pageY
    queued ||= win.requestAnimationFrame(place)
  }, { passive: true })

  useEventListener(doc, 'pointerout', (event: PointerEvent) => {
    if (mark && markOf(event.relatedTarget) !== mark) mark = null
  }, { passive: true })

  useEventListener(doc, 'focusin', (event: FocusEvent) => {
    const focused = markOf(event.target)
    if (focused && focused !== mark && (event.target as Element).matches(':focus-visible')) {
      focused.style.removeProperty('--nib-at')
    }
  })
}
