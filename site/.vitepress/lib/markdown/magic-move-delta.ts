declare global {
  interface Window {
    proseMorphProbe?: {
      cap     : number
      churn   : number
      lines   : readonly [number, number]
      morphed : boolean
      shifts  : number
    }
  }
}

// The line churn above which the surface publishes its settled output directly
// rather than morphing to it.
export const MORPH_LINE_CHURN_CAP = 60

export interface MorphMeasure {
  churn  : number
  lines  : readonly [number, number]
  shifts : number
}

// Sorts each line position into what the renderer will pay for it. A line
// holding its index and content costs nothing. One whose text differs only in
// spacing moves its tokens by transform, which composites cheaply. One whose
// content really changed pays enters, leaves and restyles, as does every line
// past the shorter state's end. Only that last tier is gated on, and reading
// the code strings rather than the machine's synced pair keeps the count
// independent of how the machine last split its tokens.
export function lineChurn(from: string, to: string): MorphMeasure {
  const before = from.split('\n')
  const after  = to.split('\n')
  const shared = Math.min(before.length, after.length)
  const bare   = (line: string) => line.replaceAll(/\s+/gu, '')
  let rewrites = before.length + after.length - shared * 2
  let shifts   = 0
  for (let line = 0; line < shared; line += 1) {
    if (before[line] === after[line]) continue
    if (bare(before[line]) === bare(after[line])) shifts += 1
    else rewrites += 1
  }
  return { churn: rewrites, lines: [before.length, after.length], shifts }
}

// Records the last morph decision for the frame probe, which reads it after a
// toggle so a measurement attributes to the path the toggle actually took.
// Nothing in the site itself reads it.
export function noteMorphDecision(measure: MorphMeasure, morphed: boolean): void {
  if (typeof window === 'undefined') return
  window.proseMorphProbe = {
    cap     : MORPH_LINE_CHURN_CAP,
    churn   : measure.churn,
    lines   : measure.lines,
    morphed,
    shifts  : measure.shifts
  }
}
