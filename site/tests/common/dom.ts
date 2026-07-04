let seq = 0

export const fire = (target: EventTarget, type: string): void => {
  target.dispatchEvent(new Event(type, { bubbles: true }))
}

export const freshName = (prefix: string): string => `${prefix}-${++seq}`
