// @vitest-environment happy-dom
import { useDismissedHover } from '../../lib/composables/use-dismissed-hover'

type Hover = ReturnType<typeof useDismissedHover>

const first  = document.createElement('a')
const second = document.createElement('a')

describe('useDismissedHover', () => {
  it('passes an element the pointer enters before any dismissal', () => {
    expect(useDismissedHover().enter(first)).toBe(false)
  })

  it('holds the element a dismissal left the pointer on', () => {
    const hover = useDismissedHover()
    hover.enter(first)
    hover.dismiss()
    expect(hover.enter(first)).toBe(true)
  })

  it.each([
    { clear: 'entering another element', move: (hover: Hover) => { hover.enter(second) } },
    { clear: 'entering no element',      move: (hover: Hover) => { hover.enter(null) } },
    { clear: 'a leave',                  move: (hover: Hover) => { hover.leave() } }
  ])('passes the dismissed element again after $clear', ({ move }) => {
    const hover = useDismissedHover()
    hover.enter(first)
    hover.dismiss()
    move(hover)
    expect(hover.enter(first)).toBe(false)
  })

  it('holds nothing when the pointer was on no element at the dismissal', () => {
    const hover = useDismissedHover()
    hover.enter(null)
    hover.dismiss()
    expect(hover.enter(null)).toBe(false)
  })
})
