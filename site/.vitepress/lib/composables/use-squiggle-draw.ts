import { ref } from 'vue'

import { nextPaint } from '../shared/paint'

// Stages the lint underlines undrawn, then lifts the class after a paint
// so their scaleX transition re-fires left to right.
export function useSquiggleDraw() {
  const undrawn = ref(false)

  async function drawSquiggles(): Promise<void> {
    if (typeof requestAnimationFrame === 'undefined') return
    undrawn.value = true
    await nextPaint()
    undrawn.value = false
  }

  return { drawSquiggles, undrawn }
}
