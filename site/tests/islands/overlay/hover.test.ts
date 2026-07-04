// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'

import { fire, freshName } from '../../common/dom'

const { floatMock, unpin } = vi.hoisted(() => {
  const stop = vi.fn()
  return { floatMock: vi.fn(() => stop), unpin: stop }
})
vi.mock('../../../src/lib/overlay/float', () => ({ float: floatMock }))

import { attachHoverOverlay } from '../../../src/lib/overlay/hover'

type Options = Parameters<typeof attachHoverOverlay>[0]

// Attaches one overlay bound to a unique selector, so listeners left on the
// shared document by earlier tests never match this test's anchor.
function overlay(over: Partial<Options> = {}): { anchor: HTMLElement, selector: string } {
  const selector = `.${freshName('anchor')}`
  const anchor   = document.createElement('button')
  document.body.append(anchor)
  anchor.className = selector.slice(1)
  attachHoverOverlay({
    panelClass : 'panel-x',
    render     : () => { const n = document.createElement('span'); n.textContent = 'C'; return n },
    selector,
    ...over
  })
  return { anchor, selector }
}

const currentPanel = () => document.querySelector('.overlay-panel')

const SHOW = 100
const HIDE = 240
const openPanel = (anchor: HTMLElement): HTMLElement => {
  fire(anchor, 'mouseover')
  vi.advanceTimersByTime(SHOW)
  return currentPanel() as HTMLElement
}

beforeEach(() => { vi.useFakeTimers(); floatMock.mockClear(); unpin.mockClear() })
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); document.body.innerHTML = '' })

describe('attachHoverOverlay', () => {
  test('opens a panel after the show delay and wires the tooltip aria', () => {
    const onOpen = vi.fn()
    const { anchor } = overlay({ onOpen })
    fire(anchor, 'mouseover')
    expect(currentPanel()).toBeNull()

    vi.advanceTimersByTime(SHOW)
    const panel = currentPanel() as HTMLElement
    expect(panel).toHaveClass('panel-x')
    expect(panel.getAttribute('role')).toBe('tooltip')
    expect(panel.textContent).toContain('C')
    expect(anchor).toHaveAttribute('aria-describedby', panel.id)
    expect(onOpen).toHaveBeenCalledWith(anchor)
    expect(floatMock).toHaveBeenCalledOnce()
  })

  test('renders nothing when render returns null', () => {
    const onOpen = vi.fn()
    const { anchor } = overlay({ onOpen, render: () => null })
    fire(anchor, 'mouseover')
    vi.advanceTimersByTime(SHOW)
    expect(currentPanel()).toBeNull()
    expect(anchor).not.toHaveAttribute('aria-describedby')
    expect(onOpen).not.toHaveBeenCalled()
  })

  test('closes after the hide delay, clearing aria and unpinning', () => {
    const onClose = vi.fn()
    const { anchor } = overlay({ onClose })
    const panel = openPanel(anchor)

    fire(anchor, 'mouseout')
    vi.advanceTimersByTime(HIDE)
    expect(panel.isConnected).toBe(false)
    expect(anchor).not.toHaveAttribute('aria-describedby')
    expect(onClose).toHaveBeenCalledWith(anchor)
    expect(unpin).toHaveBeenCalledOnce()
  })

  test('a pointer entering the panel cancels the pending hide', () => {
    const { anchor } = overlay()
    const panel = openPanel(anchor)

    panel.dispatchEvent(new Event('mouseleave'))
    panel.dispatchEvent(new Event('mouseenter'))
    vi.advanceTimersByTime(HIDE)
    expect(panel.isConnected).toBe(true)
  })

  test('a pointer leaving the panel schedules the hide', () => {
    const { anchor } = overlay()
    const panel = openPanel(anchor)

    panel.dispatchEvent(new Event('mouseleave'))
    vi.advanceTimersByTime(HIDE)
    expect(panel.isConnected).toBe(false)
  })

  test('re-hovering the open anchor does not reopen it', () => {
    const onOpen = vi.fn()
    const { anchor } = overlay({ onOpen })
    fire(anchor, 'mouseover')
    vi.advanceTimersByTime(SHOW)
    fire(anchor, 'mouseover')
    vi.advanceTimersByTime(SHOW)
    expect(onOpen).toHaveBeenCalledOnce()
  })

  test('hovering a second anchor swaps the panel onto it', () => {
    const { anchor, selector } = overlay()
    const second = document.createElement('button')
    document.body.append(second)
    second.className = selector.slice(1)

    fire(anchor, 'mouseover')
    vi.advanceTimersByTime(SHOW)
    fire(second, 'mouseover')
    vi.advanceTimersByTime(SHOW)
    expect(anchor).not.toHaveAttribute('aria-describedby')
    expect(second).toHaveAttribute('aria-describedby')
  })

  test('focus opens and blur closes the panel', () => {
    const { anchor } = overlay()
    fire(anchor, 'focusin')
    vi.advanceTimersByTime(SHOW)
    const panel = currentPanel() as HTMLElement
    expect(panel.isConnected).toBe(true)

    fire(anchor, 'focusout')
    vi.advanceTimersByTime(HIDE)
    expect(panel.isConnected).toBe(false)
  })

  test('ignores hover and focus events off any anchor', () => {
    overlay()
    const other = document.createElement('p')
    document.body.append(other)
    for (const type of ['mouseover', 'mouseout', 'focusin', 'focusout']) fire(other, type)
    vi.advanceTimersByTime(300)
    expect(currentPanel()).toBeNull()
  })

  test('a scheduled hide with nothing open is a no-op', () => {
    const { anchor } = overlay()
    fire(anchor, 'mouseout')
    vi.advanceTimersByTime(HIDE)
    expect(currentPanel()).toBeNull()
  })
})
