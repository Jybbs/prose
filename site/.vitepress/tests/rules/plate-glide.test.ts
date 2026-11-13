// @vitest-environment happy-dom
import { glide } from '../../lib/rules/plate-glide'
import { domTest } from '../dom'

const TIMING = { duration: 400, easing: 'ease' }

// Builds a plate of letters, each holding the rows its slugs name.
function plateOf(letters: Record<string, string[]>): HTMLElement {
  const plate = document.createElement('div')
  for (const [letter, slugs] of Object.entries(letters)) {
    plate.insertAdjacentHTML('beforeend', `<section class="rules-plate-letter" data-letter="${letter}">`
      + `<span class="rules-plate-cap" style="opacity: 1">${letter}</span><ul class="rules-plate-slugs">`
      + slugs.map(slug => `<li data-slug="${slug}" style="opacity: 1">${slug}</li>`).join('') + '</ul></section>')
  }
  document.body.replaceChildren(plate)
  return plate
}

// Stands each letter and row at the top `tops` gives it, one with no top
// reading no box, as a hidden element does.
function stand(plate: HTMLElement, tops: Record<string, number | null>): void {
  for (const el of plate.querySelectorAll<HTMLElement>('[data-letter], [data-slug]')) {
    const top = tops[el.dataset.letter ?? el.dataset.slug!] ?? null
    el.getBoundingClientRect = () => ({ bottom: (top ?? 0) + 20, height: 20, left: 0, top: top ?? 0, width: 100 }) as DOMRect
    el.getClientRects = () => (top === null ? [] : [el.getBoundingClientRect()]) as unknown as DOMRectList
  }
}

describe('glide', () => {
  domTest('moves each row from where it stood less its letter\'s own move, dimming one that changes letters', async () => {
    const plate = plateOf({ a: ['align', 'bare'], b: [] })
    stand(plate, { a: 0, align: 10, b: 60, bare: 30 })
    const animate = vi.spyOn(Element.prototype, 'animate')
    await glide(plate, () => {
      plate.querySelector('[data-letter="b"] ul')!.append(plate.querySelector('[data-slug="bare"]')!)
      stand(plate, { a: 0, align: 10, b: 40, bare: 50 })
      return Promise.resolve()
    }, TIMING)
    const moves = animate.mock.calls.map(([frames]) => (frames as Keyframe[])[0])
    expect(moves).toContainEqual({ transform: 'translate(0px, 20px)' })
    expect(moves).toContainEqual({ opacity: 1, transform: 'translate(0px, -40px)' })
    const bare = animate.mock.calls.find(([frames]) => (frames as Keyframe[]).length === 3)!
    expect((bare[0] as Keyframe[])[1]).toStrictEqual({ offset: 0.35, opacity: 0.45 })
  })

  domTest('cancels the moves of the press before it', async () => {
    const plate  = plateOf({ a: ['align'] })
    const cancel = vi.fn<() => void>()
    vi.spyOn(Element.prototype, 'animate').mockReturnValue({ cancel, finished: Promise.resolve() } as unknown as Animation)
    const shift = (top: number) => () => {
      stand(plate, { a: top, align: top + 10 })
      return Promise.resolve()
    }
    stand(plate, { a: 0, align: 10 })
    await glide(plate, shift(40), TIMING)
    expect(cancel).not.toHaveBeenCalled()
    await glide(plate, shift(0), TIMING)
    expect(cancel).toHaveBeenCalled()
  })

  domTest('leaves a fading copy where a letter the change hides stood', async () => {
    const plate = plateOf({ a: ['align'], b: ['bare'] })
    stand(plate, { a: 0, align: 10, b: 40, bare: 50 })
    await glide(plate, () => {
      plate.querySelector('[data-letter="b"]')!.classList.add('is-out')
      stand(plate, { a: 0, align: 10, b: null, bare: null })
      return Promise.resolve()
    }, TIMING)
    const ghost = plate.querySelector<HTMLElement>('.is-ghost')!
    expect(ghost.getAttribute('aria-hidden')).toBe('true')
    expect(ghost.classList.contains('is-out')).toBe(false)
    expect(ghost.style.top).toBe('40px')
    expect(ghost.querySelector('.rules-plate-cap')?.textContent).toBe('b')
  })

  domTest('fades in the rule and cap of a letter the change shows', async () => {
    const plate = plateOf({ a: ['align'], b: [] })
    stand(plate, { a: 0, align: 10, b: null })
    const animate = vi.spyOn(Element.prototype, 'animate')
    await glide(plate, () => Promise.resolve(stand(plate, { a: 0, align: 10, b: 40 })), TIMING)
    const frames = animate.mock.calls.map(([each]) => (each as Keyframe[])[0])
    expect(frames).toContainEqual({ borderTopColor: 'transparent' })
    expect(frames).toContainEqual({ opacity: 0 })
  })

  domTest('eases a cap and a row caught mid-fade back from where each stands', async () => {
    const plate = plateOf({ a: ['align'] })
    plate.querySelector<HTMLElement>('.rules-plate-cap')!.style.opacity = '0.5'
    plate.querySelector<HTMLElement>('[data-slug="align"]')!.style.opacity = '0.6'
    stand(plate, { a: 0, align: 10 })
    const animate = vi.spyOn(Element.prototype, 'animate')
    await glide(plate, () => Promise.resolve(), TIMING)
    const frames = animate.mock.calls.map(([each]) => (each as Keyframe[])[0])
    expect(frames).toContainEqual({ opacity: 0.5 })
    expect(frames).toContainEqual({ opacity: 0.6, transform: 'translate(0px, 0px)' })
  })

  domTest('eases the plate from the height it stood at to its new one, clipping its foot', async () => {
    const plate   = plateOf({ a: ['align'] })
    const tall    = (height: number) => { plate.getBoundingClientRect = () => ({ height, left: 0, top: 0 }) as DOMRect }
    const animate = vi.spyOn(Element.prototype, 'animate')
    stand(plate, { a: 0, align: 10 })
    tall(120)
    await glide(plate, () => Promise.resolve(tall(80)), TIMING)
    expect(animate).toHaveBeenCalledWith([
      { height: '120px', overflowY: 'clip' },
      { height: '80px', overflowY: 'clip' }
    ], TIMING)
  })

  domTest('leaves the plate\'s height to layout where the change keeps it', async () => {
    const plate   = plateOf({ a: ['align'] })
    const animate = vi.spyOn(Element.prototype, 'animate')
    stand(plate, { a: 0, align: 10 })
    await glide(plate, () => Promise.resolve(), TIMING)
    expect(animate.mock.calls.some(([frames]) => 'height' in (frames as Keyframe[])[0]!)).toBe(false)
  })

  domTest('takes its places at once under reduced motion', async ({ reducedMotion }) => {
    reducedMotion(true)
    const plate   = plateOf({ a: ['align'] })
    const animate = vi.spyOn(Element.prototype, 'animate')
    const change  = vi.fn<() => Promise<void>>(() => Promise.resolve())
    await glide(plate, change, TIMING)
    expect(change).toHaveBeenCalledOnce()
    expect(animate).not.toHaveBeenCalled()
  })
})
