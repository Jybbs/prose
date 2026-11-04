import { reducesMotion } from '../shared/paint'

// Holds the duration and curve one move of the rules index takes.
export interface GlideTiming {
  duration : number
  easing   : string
}

const LETTERS = '.rules-plate-letter:not(.is-ghost)'
const ROWS    = '.rules-plate-slugs > li[data-slug]'

const moves = new WeakMap<Element, Animation[]>()

// Moves the rules index from one arrangement to the next through `change`. It
// reads where each letter and row stands and the opacity of each row and cap,
// a move in flight included, and stops every move in flight, so a press during
// a move starts from what is on screen. After the change, each letter eases
// from where it stood and each row by what is left of its own move once its
// letter's is taken out, dimming early in the move where it changes letters. A
// letter the change hides leaves a copy of its rule and cap fading where it
// stood, and a letter the change shows fades its rule and cap in.
export async function glide(plate: HTMLElement, change: () => Promise<void>, timing: GlideTiming): Promise<void> {
  const win = plate.ownerDocument.defaultView!
  if (reducesMotion(win)) return change()

  const seen    = (el: Element) => el.getClientRects().length > 0 ? el.getBoundingClientRect() : null
  const capOf   = (letter: Element) => letter.querySelector(':scope > .rules-plate-cap')
  const fadeOf  = (el: Element | null) => el ? Number(win.getComputedStyle(el).opacity) : 1
  const slugOf  = (row: HTMLElement) => row.dataset.slug!
  const homeOf  = (row: Element) => row.closest<HTMLElement>('.rules-plate-letter')?.dataset.letter
  const letters = [...plate.querySelectorAll<HTMLElement>(LETTERS)]
  const rows    = [...plate.querySelectorAll<HTMLElement>(ROWS)]
  const base    = plate.getBoundingClientRect()
  const before  = {
    fades   : new Map(rows.map(row => [slugOf(row), fadeOf(row)])),
    homes   : new Map(rows.map(row => [slugOf(row), homeOf(row)])),
    letters : new Map(letters.map(letter => [letter.dataset.letter!, [letter, seen(letter), fadeOf(capOf(letter))] as const])),
    rows    : new Map(rows.map(row => [slugOf(row), seen(row)]))
  }

  for (const move of moves.get(plate) ?? []) move.cancel()
  const running = [] as Animation[]
  moves.set(plate, running)
  const half = { ...timing, duration: timing.duration / 2 }

  await change()

  const shift = new Map<string, [number, number]>()
  for (const letter of plate.querySelectorAll<HTMLElement>(LETTERS)) {
    const to = seen(letter)
    if (!to) continue
    const [, from, fade] = before.letters.get(letter.dataset.letter!) ?? []
    const cap            = capOf(letter)
    if (from) {
      const delta = [from.left - to.left, from.top - to.top] as [number, number]
      shift.set(letter.dataset.letter!, delta)
      if (Math.abs(delta[0]) + Math.abs(delta[1]) > 0.5) {
        running.push(letter.animate([{ transform: `translate(${delta[0]}px, ${delta[1]}px)` }, { transform: 'none' }], timing))
      }
      if (cap && fade! < 0.99) running.push(cap.animate([{ opacity: fade }, { opacity: 1 }], half))
    } else {
      const rule = win.getComputedStyle(letter).borderTopColor
      running.push(letter.animate([{ borderTopColor: 'transparent' }, { borderTopColor: rule }], half))
      if (cap) running.push(cap.animate([{ opacity: 0 }, { opacity: 1 }], half))
    }
  }

  for (const row of plate.querySelectorAll<HTMLElement>(ROWS)) {
    const from = before.rows.get(slugOf(row))
    const to   = seen(row)
    if (!from || !to) continue
    const home     = homeOf(row)!
    const [lx, ly] = shift.get(home) ?? [0, 0]
    const dx       = from.left - to.left - lx
    const dy       = from.top - to.top - ly
    const fade     = before.fades.get(slugOf(row)) ?? 1
    if (Math.abs(dx) + Math.abs(dy) <= 0.5 && fade > 0.99) continue
    const dim = before.homes.get(slugOf(row)) === home ? [] : [{ offset: 0.35, opacity: Math.min(fade, 0.45) }]
    running.push(row.animate([{ opacity: fade, transform: `translate(${dx}px, ${dy}px)` }, ...dim, { opacity: 1, transform: 'none' }], timing))
  }

  for (const [key, [letter, from, fade]] of before.letters) {
    if (!from || plate.querySelector(`${LETTERS}[data-letter="${key}"]`)?.getClientRects().length) continue
    const ghost = letter.cloneNode(false) as HTMLElement
    ghost.classList.remove('is-out')
    ghost.classList.add('is-ghost')
    delete ghost.dataset.letter
    ghost.setAttribute('aria-hidden', 'true')
    Object.assign(ghost.style, {
      height : `${from.height}px`,
      left   : `${from.left - base.left}px`,
      top    : `${from.top - base.top}px`,
      width  : `${from.width}px`
    })
    const cap = capOf(letter)
    if (cap) ghost.append(cap.cloneNode(true))
    plate.append(ghost)
    const out = ghost.animate([{ opacity: fade }, { opacity: 0 }], { ...half, fill: 'forwards' })
    void out.finished.then(() => ghost.remove(), () => ghost.remove())
  }
}
