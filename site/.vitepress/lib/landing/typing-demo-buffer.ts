import { commonPrefix } from '../shared/common-prefix'

import type * as typingDemo from './typing-demo'

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
  entries : readonly typingDemo.LandingTypingDemoEntry[],
  upTo    : number
): string {
  let text = base
  for (const entry of entries.slice(0, upTo)) {
    if (entry.kind !== 'edit') continue
    const anchorIdx = text.indexOf(entry.anchor + entry.from)
    if (anchorIdx === -1) continue
    const valueStart = anchorIdx + entry.anchor.length
    text = text.slice(0, valueStart) + entry.to + text.slice(valueStart + entry.from.length)
  }
  return text
}

// Splits an edit into the prefix `from` and `to` share and their differing
// cores, so the animation only backspaces and retypes the changed tail.
export function editPlan(from: string, to: string): {
  fromCore : string
  prefix   : string
  toCore   : string
} {
  const shared = commonPrefix(from, to)
  return { fromCore: from.slice(shared), prefix: from.slice(0, shared), toCore: to.slice(shared) }
}

export function segmentsForEdit(
  entry        : typingDemo.LandingTypingDemoEditEntry,
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

  const { prefix, fromCore, toCore } = editPlan(entry.from, entry.to)
  let editing: string
  if (phase === 'editBackspacing') {
    editing = prefix + fromCore.slice(0, fromCore.length - editProgress)
  } else if (phase === 'editTyping') {
    editing = prefix + toCore.slice(0, editProgress)
  } else {
    editing = entry.to
  }
  return { after, before, editing, editingLineAfter, editingLineBefore }
}

export function resetText(
  prelude  : string,
  rows     : readonly typingDemo.LandingTypingDemoResetRow[],
  phase    : string,
  progress : number
): string {
  let text = prelude
  for (const row of rows) {
    const partial = phase === 'resetBackspacing'
      ? row.end.slice(0, Math.max(0, row.end.length - progress))
      : row.prelude.slice(0, progress)
    const anchorIdx = text.indexOf(row.anchor + row.prelude)
    if (anchorIdx === -1) continue
    const valueStart = anchorIdx + row.anchor.length
    text = text.slice(0, valueStart) + partial + text.slice(valueStart + row.prelude.length)
  }
  return text
}
