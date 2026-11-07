const FOOT  = 10
const SHORT = 5
const TALL  = 0

// Lists each value whose output differs from the output of the value below it.
export function changeValues(steps: readonly number[]): number[] {
  return steps.flatMap((output, value) => value > 0 && output !== steps[value - 1] ? [value] : [])
}

// Places a value along the track as a percentage of its span.
export function gaugePct(steps: readonly number[], value: number): number {
  return value / (steps.length - 1) * 100
}

// Draws one tick per value across a unit-high band, a taller tick at each value
// where the output changes, the band stretching to the width of the track it sits on.
export function gaugeTicks(steps: readonly number[]): string {
  const changes = new Set(changeValues(steps))
  const ticks   = steps.map((_, value) => {
    const top = changes.has(value) ? TALL : SHORT
    return `<line data-change="${changes.has(value)}" vector-effect="non-scaling-stroke" `
      + `x1="${value}" x2="${value}" y1="${top}" y2="${FOOT}"/>`
  })
  return `<svg aria-hidden="true" overflow="visible" preserveAspectRatio="none" stroke="currentColor" `
    + `viewBox="0 0 ${steps.length - 1} ${FOOT}">${ticks.join('')}</svg>`
}
