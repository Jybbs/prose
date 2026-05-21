import type { LandingTypingDemoEditEntry, LandingTypingDemoEntry } from '../../data/landing-typing-demo.fixtures'

export interface BufferSegments {
  after             : string
  before            : string
  editing           : string
  editingLineAfter  : string
  editingLineBefore : string
}

export const EMPTY_SEGMENTS: BufferSegments = {
  after             : '',
  before            : '',
  editing           : '',
  editingLineAfter  : '',
  editingLineBefore : ''
}

export function applyCompletedEdits(
  base    : string,
  entries : readonly LandingTypingDemoEntry[],
  upTo    : number
): string {
  let text = base
  for (let i = 0; i < upTo; i++) {
    const e = entries[i]
    if (e.kind !== 'edit') continue
    const idx = text.indexOf(e.anchor + e.from)
    if (idx === -1) continue
    const valueStart = idx + e.anchor.length
    text = text.slice(0, valueStart) + e.to + text.slice(valueStart + e.from.length)
  }
  return text
}

export function segmentsForEdit(
  entry        : LandingTypingDemoEditEntry,
  text         : string,
  phase        : string,
  editProgress : number
): BufferSegments {
  const anchorIdx = text.indexOf(entry.anchor + entry.from)
  if (anchorIdx === -1) return { ...EMPTY_SEGMENTS, before: text }
  const valueStart = anchorIdx + entry.anchor.length
  const valueEnd   = valueStart + entry.from.length
  const fullBefore = text.slice(0, valueStart)
  const fullAfter  = text.slice(valueEnd)

  const lastNewline       = fullBefore.lastIndexOf('\n')
  const before            = lastNewline === -1 ? '' : fullBefore.slice(0, lastNewline + 1)
  const editingLineBefore = lastNewline === -1 ? fullBefore : fullBefore.slice(lastNewline + 1)

  const firstNewline      = fullAfter.indexOf('\n')
  const editingLineAfter  = firstNewline === -1 ? fullAfter : fullAfter.slice(0, firstNewline)
  const after             = firstNewline === -1 ? '' : fullAfter.slice(firstNewline)

  let editing: string
  if (phase === 'editTraveling') {
    editing = entry.from
  } else if (phase === 'editBackspacing') {
    editing = entry.from.slice(0, entry.from.length - editProgress)
  } else if (phase === 'editTyping') {
    editing = entry.to.slice(0, editProgress)
  } else {
    editing = entry.to
  }
  return { after, before, editing, editingLineAfter, editingLineBefore }
}
