import * as fc from 'fast-check'

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

const EMPTY_SEGMENTS: BufferSegments = {
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
    text = spliceAfterAnchor(entry.anchor, entry.to, entry.from, text)
  }
  return text
}

export const beforeOnly = (before: string): BufferSegments => ({ ...EMPTY_SEGMENTS, before })

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

export function resetText(
  phase    : string,
  prelude  : string,
  progress : number,
  rows     : readonly TypingDemoResetRow[]
): string {
  let text = prelude
  for (const row of rows) {
    const partial = phase === 'resetBackspacing'
      ? row.end.slice(0, Math.max(0, row.end.length - progress))
      : row.prelude.slice(0, progress)
    text = spliceAfterAnchor(row.anchor, partial, row.prelude, text)
  }
  return text
}

export function segmentsForEdit(
  editProgress : number,
  entry        : TypingDemoEntry,
  phase        : string,
  text         : string
): BufferSegments {
  const anchorIndex = text.indexOf(entry.anchor + entry.from)
  if (anchorIndex === -1) return beforeOnly(text)
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

// Replaces the `target` text directly after `anchor`, leaving the text
// unchanged when the anchored run is absent.
function spliceAfterAnchor(
  anchor      : string,
  replacement : string,
  target      : string,
  text        : string
): string {
  const index = text.indexOf(anchor + target)
  if (index === -1) return text
  const start = index + anchor.length
  return text.slice(0, start) + replacement + text.slice(start + target.length)
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const edit = (anchor: string, from: string, to: string): TypingDemoEntry =>
    ({ anchor, from, kind: 'edit', slug: anchor.trim(), to })

  describe('editPlan', () => {
    test.each([
      { name: 'splits a fully differing pair', from: 'false', to: 'true', prefix: '',     fromCore: 'false', toCore: 'true' },
      { name: 'keeps the shared prefix',       from: 'abc',   to: 'abd',  prefix: 'ab',   fromCore: 'c',     toCore: 'd'    },
      { name: 'handles an identical pair',     from: 'same',  to: 'same', prefix: 'same', fromCore: '',      toCore: ''     },
      { name: 'handles a pure extension',      from: 'ab',    to: 'abcd', prefix: 'ab',   fromCore: '',      toCore: 'cd'   }
    ])('$name', ({ from, to, prefix, fromCore, toCore }) => {
      expect(editPlan(from, to)).toEqual({ fromCore, prefix, toCore })
    })

    test('reassembles into the originals', () => {
      fc.assert(fc.property(fc.string(), fc.string(), (from, to) => {
        const { prefix, fromCore, toCore } = editPlan(from, to)
        expect(prefix + fromCore).toBe(from)
        expect(prefix + toCore).toBe(to)
      }))
    })
  })

  describe('beforeOnly', () => {
    test('fills the before segment and blanks the rest', () => {
      expect(beforeOnly('x = 1')).toEqual({
        after: '', before: 'x = 1', editing: '', editingLineAfter: '', editingLineBefore: ''
      })
    })
  })

  describe('applyCompletedEdits', () => {
    const base    = 'a = false\nb = no'
    const entries = [edit('a = ', 'false', 'true'), edit('b = ', 'no', 'yes')]

    test.each([
      { name: 'returns the base untouched at zero', upTo: 0, expected: 'a = false\nb = no' },
      { name: 'applies the first edit',             upTo: 1, expected: 'a = true\nb = no'  },
      { name: 'applies every edit',                 upTo: 2, expected: 'a = true\nb = yes' }
    ])('$name', ({ upTo, expected }) => {
      expect(applyCompletedEdits(base, entries, upTo)).toBe(expected)
    })

    test('leaves the text unchanged when the anchor is absent', () => {
      expect(applyCompletedEdits(base, [edit('z = ', 'x', 'y')], 1)).toBe(base)
    })
  })

  describe('resetText', () => {
    const prelude = 'a = false\n'
    const rows    = [{ anchor: 'a = ', end: 'true', prelude: 'false' }]

    test.each([
      { name: 'truncates the end while backspacing',       phase: 'resetBackspacing', progress: 1,  expected: 'a = tru\n' },
      { name: 'blanks the value at full backspace',        phase: 'resetBackspacing', progress: 10, expected: 'a = \n'    },
      { name: 'retypes the prelude while typing',          phase: 'resetTyping',      progress: 2,  expected: 'a = fa\n'  }
    ])('$name', ({ phase, progress, expected }) => {
      expect(resetText(phase, prelude, progress, rows)).toBe(expected)
    })
  })

  describe('segmentsForEdit', () => {
    const entry = edit('x = ', 'false', 'true')

    test.each([
      { name: 'backspaces the differing core', phase: 'editBackspacing', progress: 2, editing: 'fal'  },
      { name: 'types the new core',            phase: 'editTyping',      progress: 2, editing: 'tr'   },
      { name: 'shows the final value at rest',  phase: 'holdAfterTyped',  progress: 0, editing: 'true' }
    ])('$name', ({ phase, progress, editing }) => {
      expect(segmentsForEdit(progress, entry, phase, 'x = false\ny = no')).toEqual({
        after: '\ny = no', before: '', editing, editingLineAfter: '', editingLineBefore: 'x = '
      })
    })

    test('splits the surrounding lines around the edit', () => {
      expect(segmentsForEdit(0, entry, 'editTyping', 'top\nx = false\nend')).toMatchObject({
        after: '\nend', before: 'top\n', editingLineAfter: '', editingLineBefore: 'x = '
      })
    })

    test('leaves no trailing line when the edit ends the text', () => {
      expect(segmentsForEdit(0, entry, 'editTyping', 'x = false')).toMatchObject({
        after: '', editingLineAfter: ''
      })
    })

    test('falls back to before-only when the anchor is missing', () => {
      expect(segmentsForEdit(0, edit('x = ', 'nope', 'y'), 'editTyping', 'x = false')).toEqual({
        after: '', before: 'x = false', editing: '', editingLineAfter: '', editingLineBefore: ''
      })
    })
  })
}
