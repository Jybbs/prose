// The element the event targets, resolved through `closest`, or `null` when
// the target is not an element or nothing matches.
export function closestFrom(event: Event, selector: string): HTMLElement | null {
  return event.target instanceof Element ? event.target.closest<HTMLElement>(selector) : null
}
