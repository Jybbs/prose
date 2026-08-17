// The panel component type and render options shared by every
// `ShikiMagicMovePrecompiled` mount, kept apart from the precompiler so
// a static import stays out of the shiki chunk.
export type MagicMovePanel = typeof import('@shikijs/magic-move/vue').ShikiMagicMovePrecompiled | null

// `ShikiMagicMovePrecompiled` re-syncs token keys at display time with
// these same options, so the differ pair sits beside the render knobs
// and boundary-straddling tokens slide rather than cross-fading. The
// zero container delay resizes the panel concurrently with the moves.
export function magicMoveOptions(duration: number, stagger = 3) {
  return {
    containerStyle  : false,
    delayContainer  : 0,
    delayMove       : 0,
    duration,
    enhanceMatching : true,
    splitTokens     : true,
    stagger
  }
}
