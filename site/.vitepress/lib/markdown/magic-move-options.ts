// The panel component type and render options shared by every
// `ShikiMagicMovePrecompiled` mount, kept apart from the precompiler so
// a static import stays out of the shiki chunk.
export type MagicMovePanel = typeof import('@shikijs/magic-move/vue').ShikiMagicMovePrecompiled | null

const DELAY_ENTER       = 0.7
const DELAY_LEAVE       = 0.1
const WATCHDOG_GRACE_MS = 250

// `ShikiMagicMovePrecompiled` re-syncs token keys at display time with
// these same options, so the differ pair sits beside the render knobs
// and boundary-straddling tokens slide rather than cross-fading. The
// zero container delay resizes the panel concurrently with the moves.
export function magicMoveOptions(duration: number, stagger = 3) {
  return {
    containerStyle  : false,
    delayContainer  : 0,
    delayEnter      : DELAY_ENTER,
    delayLeave      : DELAY_LEAVE,
    delayMove       : 0,
    duration,
    enhanceMatching : true,
    splitTokens     : true,
    stagger
  }
}

// The enter and leave transitions start a fraction of the duration late, so a
// morph runs until the longest-delayed one finishes rather than until
// `duration` alone elapses. A caller restoring its own surface when no `end`
// arrives waits out that whole envelope plus a grace.
export function magicMoveWatchdogMs(duration: number): number {
  return duration * (1 + Math.max(DELAY_ENTER, DELAY_LEAVE)) + WATCHDOG_GRACE_MS
}
