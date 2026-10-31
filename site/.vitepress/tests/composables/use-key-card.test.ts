// @vitest-environment happy-dom
import { nextTick, shallowRef } from 'vue'

import { coverOf, placeCard, useKeyCard } from '../../lib/composables/use-key-card'
import { domTest, mountSetup, stubRect }  from '../dom'

const box = (left: number, top: number, width: number, height: number) =>
  ({ bottom: top + height, height, left, right: left + width, top, width })

describe('placeCard', () => {
  it('hangs the card at its key\'s right where the page has room', () => {
    expect(placeCard(box(100, 50, 200, 24), 300, 1440, 80)).toStrictEqual({ gap: 14, side: 'right', x: 314, y: 50 })
  })

  it('hangs the card at its key\'s left where the right runs past the page', () => {
    expect(placeCard(box(900, 50, 200, 24), 300, 1200, 80)).toStrictEqual({ gap: 14, side: 'left', x: 586, y: 50 })
  })

  it('sets the card under its key where neither side has room, at the next key down where nearer', () => {
    expect(placeCard(box(16, 50, 358, 24), 300, 390, 80)).toStrictEqual({ gap: 6, side: 'bottom', x: 16, y: 80 })
    expect(placeCard(box(16, 50, 358, 24), 300, 390, 200)).toStrictEqual({ gap: 14, side: 'bottom', x: 16, y: 88 })
  })
})

describe('coverOf', () => {
  const rows = [box(0, 100, 200, 24), box(0, 124, 200, 24), box(400, 90, 200, 40)]

  it('lengthens the card to the foot of each row its foot would cut', () => {
    expect(coverOf(rows, 50, 0, 300, 110)).toBe(14)
  })

  it('leaves a card whose foot clears every row, and a row beside it, as it is', () => {
    expect(coverOf(rows, 50, 0, 300, 100)).toBe(0)
    expect(coverOf(rows, 650, 0, 300, 105)).toBe(0)
  })

  it('lengthens again where the first lengthening meets the next row', () => {
    expect(coverOf([box(0, 100, 200, 24), box(0, 123, 200, 24)], 50, 0, 300, 110)).toBe(37)
  })
})

describe('useKeyCard', () => {
  // Builds a stage holding two keys and a card, `useKeyCard` placing the card.
  function stageOf() {
    const stage = document.createElement('div')
    stage.innerHTML = '<a class="key">a</a><a class="key">b</a><aside class="card"></aside>'
    document.body.replaceChildren(stage)
    stubRect(stage, { left: 0, top: 0 })
    const [a, b] = stage.querySelectorAll<HTMLElement>('.key')
    stubRect(a!, box(10, 10, 100, 20))
    stubRect(b!, box(10, 40, 100, 20))
    const card = stage.querySelector<HTMLElement>('.card')!
    Object.defineProperty(card, 'offsetWidth', { value: 300 })
    Object.defineProperty(card, 'offsetHeight', { value: 50 })
    Object.defineProperty(document.documentElement, 'clientWidth', { configurable: true, value: 1440 })
    const api = mountSetup(() => useKeyCard<string>(shallowRef(stage), shallowRef(card), '.key', 220))
    return { a: a!, api, b: b!, card }
  }

  domTest('shows the card beside the key a reader reaches, in place on its first show', async () => {
    const { a, api, card } = stageOf()
    await api.show(a, 'first')
    expect(api.shown.value).toStrictEqual({ item: 'first', key: a })
    expect(card.dataset.side).toBe('right')
    expect(card.style.transform).toBe('translate(124px, 10px)')
    expect(card.style.getPropertyValue('--key-h')).toBe('20px')
    expect('snap' in card.dataset).toBe(false)
  })

  domTest('travels to the next key, easing its height to the new content', async () => {
    const { a, api, b, card } = stageOf()
    await api.show(a, 'first')
    const animate = vi.spyOn(card, 'animate').mockReturnValue({} as Animation)
    await api.show(b, 'second')
    expect(card.style.transform).toBe('translate(124px, 40px)')
    expect(animate).toHaveBeenCalled()
  })

  domTest('hides the card once the reader leaves, unless the card keeps it', async () => {
    vi.useFakeTimers()
    const { a, api } = stageOf()
    await api.show(a, 'first')
    api.hide()
    api.keep()
    vi.advanceTimersByTime(300)
    expect(api.shown.value).not.toBeNull()
    api.hide()
    vi.advanceTimersByTime(300)
    await nextTick()
    expect(api.shown.value).toBeNull()
    vi.useRealTimers()
  })
})
