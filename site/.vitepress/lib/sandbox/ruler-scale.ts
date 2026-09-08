import { clamp } from '@vueuse/core'

export const LENGTH_MAX = 180
export const LENGTH_MIN = 30

const CHIP_FALLBACK  = 72
const CLEARANCE      = 8
const SPAN           = LENGTH_MAX - LENGTH_MIN
const TRACK_FALLBACK = 480

interface RulerStop {
  key : string
  pct : number
}

// Rounds a line length onto the rail and holds it between the rail's ends.
export function clampLength(value: number): number {
  return clamp(Math.round(value), LENGTH_MIN, LENGTH_MAX)
}

// Assigns each stop the lowest tier whose occupant sits clear of it, reading
// left to right, so a chip steps up only while it meets another and drops
// back once clear.
export function packTiers(
  stops      : readonly RulerStop[],
  trackWidth : number,
  chipWidths : ReadonlyMap<string, number>
): Map<string, number> {
  const width  = (key: string): number => chipWidths.get(key) ?? CHIP_FALLBACK
  const track  = trackWidth || TRACK_FALLBACK
  const placed : RulerStop[] = []
  const tiers  = new Map<string, number>()
  for (const stop of [...stops].sort((a, b) => a.pct - b.pct)) {
    let tier = 0
    while (tier < placed.length && collides(placed[tier], stop, track, width)) tier += 1
    placed[tier] = stop
    tiers.set(stop.key, tier)
  }
  return tiers
}

// Converts a line length to its percentage along the rail.
export function pctOfLength(value: number): number {
  return ((clampLength(value) - LENGTH_MIN) / SPAN) * 100
}

// Maps a keyboard step onto a line length, returning `null` for a key the
// rail does not handle, where shift widens an arrow step to ten.
export function steppedLength(key: string, shiftKey: boolean, value: number): number | null {
  const step = shiftKey ? 10 : 1
  switch (key) {
    case 'ArrowDown':
    case 'ArrowLeft':  return value - step
    case 'ArrowRight':
    case 'ArrowUp':    return value + step
    case 'PageDown':   return value - 10
    case 'PageUp':     return value + 10
    case 'End':        return LENGTH_MAX
    case 'Home':       return LENGTH_MIN
    default:           return null
  }
}

// Places a stop's chip where it renders, mirroring the CSS `translateX`
// clamp that pins a chip at a track edge.
function chipCenter(
  stop  : RulerStop,
  track : number,
  width : (key: string) => number
): number {
  const chip  = width(stop.key)
  const x     = (stop.pct / 100) * track
  const shift = Math.max(-x, Math.min(-chip / 2, track - x - chip))
  return x + shift + chip / 2
}

// Reports two chips as colliding when their rendered centers sit closer than
// the pair's mean measured width plus a clearance.
function collides(
  a     : RulerStop,
  b     : RulerStop,
  track : number,
  width : (key: string) => number
): boolean {
  const clear = (width(a.key) + width(b.key)) / 2 + CLEARANCE
  return Math.abs(chipCenter(a, track, width) - chipCenter(b, track, width)) < clear
}
