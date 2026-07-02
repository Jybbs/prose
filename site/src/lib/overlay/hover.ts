import type { Placement } from '@floating-ui/dom'

import { float }       from './float'
import { closestFrom } from '../shared/dom/closest-from'
import { el }          from '../shared/dom/el'

export interface HoverOverlayOptions {
  gapPx       ?: number
  hideDelayMs ?: number
  onClose     ?: (anchor: HTMLElement) => void
  onOpen      ?: (anchor: HTMLElement) => void
  panelClass   : string
  placement   ?: Placement
  render       : (anchor: HTMLElement) => Node | null
  selector     : string
  showDelayMs ?: number
}

interface OpenPanel {
  anchor : HTMLElement
  panel  : HTMLElement
  unpin  : () => void
}

let panelCount = 0

// One document-level listener set per surface. Hovering or focusing a node
// matching `selector` opens a floating panel filled by `render`, the hide
// grace letting the pointer cross the gap onto the panel and hold it open.
export function attachHoverOverlay(options: HoverOverlayOptions): void {
  const { hideDelayMs = 240, showDelayMs = 100 } = options
  const panelId = `overlay-panel-${++panelCount}`

  let open: OpenPanel | null = null
  let showTimer = 0
  let hideTimer = 0

  function close(): void {
    if (open === null) return
    open.unpin()
    open.anchor.removeAttribute('aria-describedby')
    open.panel.remove()
    options.onClose?.(open.anchor)
    open = null
  }

  function show(anchor: HTMLElement): void {
    close()
    const content = options.render(anchor)
    if (content === null) return
    const panel = el('div', `overlay-panel ${options.panelClass}`)
    panel.id   = panelId
    panel.role = 'tooltip'
    panel.append(content)
    panel.addEventListener('mouseenter', () => window.clearTimeout(hideTimer))
    panel.addEventListener('mouseleave', scheduleHide)
    document.body.append(panel)
    anchor.setAttribute('aria-describedby', panel.id)
    open = { anchor, panel, unpin: float(anchor, panel, options) }
    options.onOpen?.(anchor)
  }

  function scheduleHide(): void {
    window.clearTimeout(showTimer)
    window.clearTimeout(hideTimer)
    hideTimer = window.setTimeout(close, hideDelayMs)
  }

  function scheduleShow(anchor: HTMLElement): void {
    window.clearTimeout(showTimer)
    window.clearTimeout(hideTimer)
    if (open?.anchor === anchor) return
    showTimer = window.setTimeout(() => show(anchor), showDelayMs)
  }

  document.addEventListener('mouseover', event => {
    const anchor = closestFrom(event, options.selector)
    if (anchor) scheduleShow(anchor)
  })
  document.addEventListener('mouseout', event => {
    if (closestFrom(event, options.selector)) scheduleHide()
  })
  document.addEventListener('focusin', event => {
    const anchor = closestFrom(event, options.selector)
    if (anchor) scheduleShow(anchor)
  })
  document.addEventListener('focusout', event => {
    if (closestFrom(event, options.selector)) scheduleHide()
  })
}
