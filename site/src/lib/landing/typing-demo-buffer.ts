export interface TypingDemoEntry {
  anchor : string
  from   : string
  kind   : 'edit'
  slug   : string
  tail  ?: string
  to     : string
}

export interface TypingDemoResetRow {
  anchor  : string
  end     : string
  prelude : string
}

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
  entries : readonly TypingDemoEntry[],
  upTo    : number
): string {
  let text = base
  for (const entry of entries.slice(0, upTo)) {
    const index = text.indexOf(entry.anchor + entry.from)
    if (index === -1) continue
    const valueStart = index + entry.anchor.length
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
  let shared = 0
  const max = Math.min(from.length, to.length)
  while (shared < max && from[shared] === to[shared]) shared += 1
  return { fromCore: from.slice(shared), prefix: from.slice(0, shared), toCore: to.slice(shared) }
}

export function segmentsForEdit(
  entry        : TypingDemoEntry,
  text         : string,
  phase        : string,
  editProgress : number
): BufferSegments {
  const anchorIndex = text.indexOf(entry.anchor + entry.from)
  if (anchorIndex === -1) return { ...EMPTY_SEGMENTS, before: text }
  const valueStart = anchorIndex + entry.anchor.length
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
  rows     : readonly TypingDemoResetRow[],
  phase    : string,
  progress : number
): string {
  let text = prelude
  for (const row of rows) {
    const partial = phase === 'resetBackspacing'
      ? row.end.slice(0, Math.max(0, row.end.length - progress))
      : row.prelude.slice(0, progress)
    const anchorIndex = text.indexOf(row.anchor + row.prelude)
    if (anchorIndex === -1) continue
    const valueStart = anchorIndex + row.anchor.length
    text = text.slice(0, valueStart) + partial + text.slice(valueStart + row.prelude.length)
  }
  return text
}
