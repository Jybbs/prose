import { defaultWindow, useEventListener } from '@vueuse/core'
import { nextTick, shallowRef }            from 'vue'
import type { Ref, ShallowRef }            from 'vue'

import { millis, morphEase, reducesMotion, snapInPlace } from '../../kit/drawing/paint'

type Side = 'bottom' | 'left' | 'right'

// Holds where a card stands in viewport coordinates, the side of its key it
// hangs from, and the gap its band crosses to reach the key.
export interface Placement {
  gap  : number
  side : Side
  x    : number
  y    : number
}

interface Box {
  bottom : number
  left   : number
  right  : number
  top    : number
}

// Holds the gap a card keeps from its key and the margin it keeps from either
// side of the page.
export interface CardFit {
  gap    : number
  margin : number
}

const FIT: CardFit = { gap: 14, margin: 8 }

// Places a card `width` wide at its key's right, or at its left where the
// right runs past the page, or else under the key at the key's width, the gap
// below the key or the top of the next key below it where that is nearer, so
// the card covers that key whole.
export function placeCard(
  key     : Box,
  width   : number,
  page    : number,
  nextTop : number,
  fit     : CardFit = FIT
): Placement {
  if (key.right + fit.gap + width <= page - fit.margin) {
    return { gap: fit.gap, side: 'right', x: key.right + fit.gap, y: key.top }
  }
  if (key.left - fit.gap - width >= fit.margin) {
    return { gap: fit.gap, side: 'left', x: key.left - fit.gap - width, y: key.top }
  }
  const y = Math.min(key.bottom + fit.gap, nextTop)
  return { gap: y - key.bottom, side: 'bottom', x: Math.max(fit.margin, key.left), y }
}

// Returns how far a card at `x`, `y` lengthens so its foot clears every row
// of the list behind it that the foot would cut through, ending at the foot
// of the lowest such row, over passes enough for one lengthening to meet the
// next row.
export function coverOf(rows: readonly Box[], x: number, y: number, width: number, height: number): number {
  let extra = 0
  for (let pass = 0; pass < 3; pass++) {
    const foot = y + height + extra
    const cut  = rows.filter(row =>
      row.right > x + 1 && row.left < x + width - 1 && row.top < foot - 0.5 && row.bottom > foot + 0.5)
    if (cut.length === 0) break
    extra += Math.ceil(Math.max(...cut.map(row => row.bottom)) - foot)
  }
  return extra
}

// Shows one card for every key of a list inside `stage`, the card hanging
// from the key a reader points at or focuses and traveling between keys on
// its own transition, which `data-snap` drops for the card's first place and
// for a place after a change of width. `rows` names the elements whose rows
// the card lengthens to cover, and leaving a key or the card hides it after
// `delay` milliseconds unless the reader reaches another.
export function useKeyCard<T>(
  stage : Readonly<Ref<HTMLElement | null>>,
  card  : Readonly<Ref<HTMLElement | null>>,
  rows  : string,
  delay : number
): {
  hide  : () => void
  keep  : () => void
  show  : (key: HTMLElement, item: T) => Promise<void>
  shown : ShallowRef<{ item: T, key: HTMLElement } | null>
} {
  const shown = shallowRef<{ item: T, key: HTMLElement } | null>(null)
  let timer   = 0
  let width   = 0

  const place = (snap: boolean): void => {
    const el  = card.value
    const box = stage.value?.getBoundingClientRect()
    const key = shown.value?.key
    if (!el || !box || !key) return
    const page = document.documentElement.clientWidth
    const rect = key.getBoundingClientRect()
    el.style.setProperty('--key-h', `${rect.height}px`)
    el.style.setProperty('--key-w', `${rect.width}px`)
    el.style.setProperty('--cover', '0px')
    delete el.dataset.side
    const lists = [...stage.value!.querySelectorAll(rows)].map(row => row.getBoundingClientRect())
    const next  = Math.min(Infinity, ...lists.map(row => row.top).filter(top => top >= rect.bottom - 1))
    const at    = placeCard(rect, el.offsetWidth, page, Math.floor(next))
    el.dataset.side = at.side
    el.style.setProperty('--gap', `${at.gap}px`)
    snapInPlace(el, snap || page !== width, () => {
      el.style.transform = `translate(${Math.round(at.x - box.left)}px, ${Math.round(at.y - box.top)}px)`
      el.style.setProperty('--cover', `${coverOf(lists, at.x, at.y, el.offsetWidth, el.offsetHeight)}px`)
    })
    width = page
  }

  const keep = (): void => { window.clearTimeout(timer) }

  const hide = (): void => {
    keep()
    timer = window.setTimeout(() => { shown.value = null }, delay)
  }

  const show = async (key: HTMLElement, item: T): Promise<void> => {
    keep()
    const travel = shown.value !== null && shown.value.key !== key
    const from   = card.value?.getBoundingClientRect().height ?? 0
    shown.value  = { item, key }
    await nextTick()
    place(!travel)
    const el = card.value
    if (!travel || !el || reducesMotion()) return
    const grow = { duration: millis(window.getComputedStyle(el).transitionDuration), easing: morphEase() }
    el.animate([{ height: `${from}px` }, { height: `${el.getBoundingClientRect().height}px` }], grow)
    for (const part of el.children) part.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 180, easing: 'ease-out' })
  }

  useEventListener(defaultWindow, 'resize', () => place(false))

  return { hide, keep, show, shown }
}
