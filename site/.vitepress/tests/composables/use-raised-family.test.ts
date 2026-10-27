// @vitest-environment happy-dom
import { useRaisedFamily } from '../../lib/composables/use-raised-family'
import { mountSetup }      from '../dom'

// Builds the focus event a raise reads, its target matching `:focus-visible`
// only where the browser would draw a focus ring.
const focusing = (visible: boolean): FocusEvent =>
  ({ target: { matches: (selector: string) => visible && selector === ':focus-visible' } }) as never

const escape = () => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))

describe('useRaisedFamily', () => {
  it('shows no family at rest', () => {
    expect(mountSetup(useRaisedFamily).shown.value).toBeNull()
  })

  it('ranks the pointer over the focus and the focus over the pin', () => {
    const raise = mountSetup(useRaisedFamily)
    raise.on('lint').onClick()
    raise.on('docs').onFocusin(focusing(true))
    raise.on('layout').onMouseenter()
    expect(raise.shown.value).toBe('layout')
    raise.on('layout').onMouseleave()
    expect(raise.shown.value).toBe('docs')
    raise.on('docs').onFocusout()
    expect(raise.shown.value).toBe('lint')
  })

  it('counts only a focus the browser draws a ring for', () => {
    const raise = mountSetup(useRaisedFamily)
    raise.on('docs').onFocusin(focusing(false))
    expect(raise.shown.value).toBeNull()
  })

  it('releases a pin on a second click of the same family and moves it on another', () => {
    const raise = mountSetup(useRaisedFamily)
    raise.on('ordering').onClick()
    raise.on('alignment').onClick()
    expect(raise.pinned.value).toBe('alignment')
    raise.on('alignment').onClick()
    expect(raise.pinned.value).toBeNull()
  })

  it('releases the pin and the focus on Escape, leaving the pointer', () => {
    const raise = mountSetup(useRaisedFamily)
    raise.on('lint').onClick()
    raise.on('docs').onFocusin(focusing(true))
    escape()
    expect(raise.shown.value).toBeNull()
    raise.on('layout').onMouseenter()
    escape()
    expect(raise.shown.value).toBe('layout')
  })
})
