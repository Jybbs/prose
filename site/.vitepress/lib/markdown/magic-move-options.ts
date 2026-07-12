// The panel component type and render options shared by every
// `ShikiMagicMovePrecompiled` mount, kept apart from the precompiler so
// a static import stays out of the shiki chunk.
export type MagicMovePanel = typeof import('@shikijs/magic-move/vue').ShikiMagicMovePrecompiled | null

export function magicMoveOptions(duration: number) {
  return { containerStyle: false, delayMove: 0, duration, stagger: 3 }
}
