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

// The span of the value following `anchor`, or null when the anchor and its
// current value are absent.
function anchorRange(
  text   : string,
  anchor : string,
  from   : string
): { end: number, start: number } | null {
  const anchorIdx = text.indexOf(anchor + from)
  if (anchorIdx === -1) return null
  const start = anchorIdx + anchor.length
  return { end: start + from.length, start }
}

function spliceAfterAnchor(text: string, anchor: string, from: string, to: string): string {
  const range = anchorRange(text, anchor, from)
  if (!range) return text
  return text.slice(0, range.start) + to + text.slice(range.end)
}

export function applyCompletedEdits(
  base    : string,
  entries : readonly typingDemo.LandingTypingDemoEntry[],
  upTo    : number
): string {
  return entries.slice(0, upTo).reduce((text, entry) => entry.kind === 'edit'
    ? spliceAfterAnchor(text, entry.anchor, entry.from, entry.to)
    : text, base)
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

export function resetText(
  prelude  : string,
  rows     : readonly typingDemo.LandingTypingDemoResetRow[],
  phase    : string,
  progress : number
): string {
  return rows.reduce((text, row) => {
    const partial = phase === 'resetBackspacing'
      ? row.end.slice(0, Math.max(0, row.end.length - progress))
      : row.prelude.slice(0, progress)
    return spliceAfterAnchor(text, row.anchor, row.prelude, partial)
  }, prelude)
}

export function segmentsForEdit(
  entry        : typingDemo.LandingTypingDemoEditEntry,
  text         : string,
  phase        : string,
  editProgress : number
): BufferSegments {
  const range = anchorRange(text, entry.anchor, entry.from)
  if (!range) return { ...EMPTY_SEGMENTS, before: text }
  const fullBefore = text.slice(0, range.start)
  const fullAfter  = text.slice(range.end)

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
