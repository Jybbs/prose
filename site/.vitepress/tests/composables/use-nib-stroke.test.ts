// @vitest-environment happy-dom
import { NIB_MARKS, useNibStroke } from '../../lib/composables/use-nib-stroke'
import { mountSetup, nextFrame }   from '../dom'

// Builds a page of one body link standing on two lines, 100px and 60px long,
// the second line starting at the left edge 20px below the first.
function linkOf(): HTMLElement {
  document.body.innerHTML = '<p><a class="body-link" href="/a">two lines of link words</a></p>'
  const link = document.querySelector<HTMLElement>('a')!
  link.style.fontSize = '16px'
  link.getClientRects = () => [
    { bottom: 20, left: 40, top: 0, width: 100 },
    { bottom: 40, left: 0,  top: 20, width: 60 }
  ] as unknown as DOMRectList
  return link
}

// Dispatches a pointer event at page coordinates `x`, `y`, which happy-dom
// leaves at zero for a synthetic event.
const point = (target: Element, type: string, x: number, y: number) => {
  const event = new PointerEvent(type, { bubbles: true })
  Object.defineProperties(event, { pageX: { value: x }, pageY: { value: y } })
  target.dispatchEvent(event)
}

describe('useNibStroke', () => {
  it('matches a link\'s words, a link\'s code, and a nib-stroke element', () => {
    document.body.innerHTML = '<a class="body-link">w</a><a class="body-link"><code>c</code></a><span class="nib-stroke">s</span>'
    expect([...document.querySelectorAll(NIB_MARKS)].map(el => el.textContent)).toStrictEqual(['w', 'c', 's'])
  })

  it('sets the stroke under the pointer along the line it rests on, half a stroke inside the line\'s ends', async () => {
    const link = linkOf()
    mountSetup(() => useNibStroke(document))
    point(link, 'pointerover', 100, 10)
    expect(link.style.getPropertyValue('--nib-at')).toBe('calc(60px - var(--nib-swell) * var(--prose-size-2-5em) / 2)')
    point(link, 'pointermove', 5, 30)
    await nextFrame()
    expect(link.style.getPropertyValue('--nib-at')).toBe('calc(20px - var(--nib-swell) * var(--prose-size-2-5em) / 2)')
  })

  it('returns the stroke to the middle of each line under a keyboard focus', () => {
    const link = linkOf()
    mountSetup(() => useNibStroke(document))
    link.style.setProperty('--nib-at', '10px')
    Object.defineProperty(link, 'matches', {
      value: (selector: string) => selector === ':focus-visible' || Element.prototype.matches.call(link, selector)
    })
    link.dispatchEvent(new FocusEvent('focusin', { bubbles: true }))
    expect(link.style.getPropertyValue('--nib-at')).toBe('')
  })

  it('finds the mark a link holds when the pointer rests on the link itself', () => {
    document.body.innerHTML = '<a class="rules-plate-go" href="/a"><span class="nib-stroke">→</span></a>'
    const mark = document.querySelector<HTMLElement>('.nib-stroke')!
    mark.getClientRects = () => [{ bottom: 20, left: 0, top: 0, width: 40 }] as unknown as DOMRectList
    mountSetup(() => useNibStroke(document))
    point(document.querySelector('a')!, 'pointerover', 20, 10)
    expect(mark.style.getPropertyValue('--nib-at')).toMatch(/^calc\(20px/)
  })

  it('keeps the stroke\'s place under a focus that draws no ring', () => {
    const link = linkOf()
    mountSetup(() => useNibStroke(document))
    link.style.setProperty('--nib-at', '10px')
    link.dispatchEvent(new FocusEvent('focusin', { bubbles: true }))
    expect(link.style.getPropertyValue('--nib-at')).toBe('10px')
  })

  it('forgets the mark once the pointer leaves it', async () => {
    const link = linkOf()
    mountSetup(() => useNibStroke(document))
    point(link, 'pointerover', 100, 10)
    link.dispatchEvent(new PointerEvent('pointerout', { bubbles: true, relatedTarget: document.body }))
    point(link, 'pointermove', 5, 30)
    await nextFrame()
    expect(link.style.getPropertyValue('--nib-at')).toBe('calc(60px - var(--nib-swell) * var(--prose-size-2-5em) / 2)')
  })
})
