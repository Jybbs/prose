// @vitest-environment happy-dom
import { useDismissedHover } from '../../lib/composables/use-dismissed-hover'

type Hover = ReturnType<typeof useDismissedHover>

const first  = document.createElement('a')
const second = document.createElement('a')

// Enters `first` and dismisses its card.
function dismissed(): Hover {
  const hover = useDismissedHover()
  hover.enter(first)
  hover.dismiss()
  return hover
}

describe('useDismissedHover', () => {
  it('passes an element the pointer enters before any dismissal', () => {
    expect(useDismissedHover().enter(first)).toBe(false)
  })

  it('holds the element a dismissal left the pointer on', () => {
    expect(dismissed().enter(first)).toBe(true)
  })

  it.each([
    { hold: 'entering another element and back', move: (hover: Hover) => { hover.enter(second) } },
    { hold: 'entering no element and back',      move: (hover: Hover) => { hover.enter(null) } },
    { hold: 'a move inside it',                  move: (hover: Hover) => { hover.move(first) } }
  ])('holds the dismissed element after $hold', ({ move }) => {
    const hover = dismissed()
    move(hover)
    expect(hover.enter(first)).toBe(true)
  })

  it.each([
    { clear: 'a move onto another element', move: (hover: Hover) => { hover.move(second) } },
    { clear: 'a move onto no element',      move: (hover: Hover) => { hover.move(null) } },
    { clear: 'a leave',                     move: (hover: Hover) => { hover.leave() } }
  ])('passes the dismissed element again after $clear', ({ move }) => {
    const hover = dismissed()
    move(hover)
    expect(hover.enter(first)).toBe(false)
  })

  it('holds nothing when the pointer was on no element at the dismissal', () => {
    const hover = useDismissedHover()
    hover.enter(null)
    hover.dismiss()
    expect(hover.enter(null)).toBe(false)
  })

  it('passes the element a leave preceded at the next dismissal', () => {
    const hover = useDismissedHover()
    hover.enter(first)
    hover.leave()
    hover.dismiss()
    expect(hover.enter(first)).toBe(false)
  })
})
