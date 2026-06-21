import { fc, test } from '@fast-check/vitest'

import type {
  LandingTypingDemoEntry, LandingTypingDemoResetRow
} from '../lib/landing/typing-demo'
import {
  applyCompletedEdits,
  editPlan,
  EMPTY_SEGMENTS,
  resetText,
  segmentsForEdit
} from '../lib/landing/typing-demo-buffer'

describe('editPlan', () => {
  it.each([
    ['false', 'true',  { fromCore: 'false', prefix: '',    toCore: 'true' }],
    ['abcr',  'abcz',  { fromCore: 'r',     prefix: 'abc', toCore: 'z' }],
    ['abc',   'abcde', { fromCore: '',      prefix: 'abc', toCore: 'de' }],
    ['abcde', 'abc',   { fromCore: 'de',    prefix: 'abc', toCore: '' }]
  ])('splits %s and %s on their shared prefix', (from, to, expected) => {
    expect(editPlan(from, to)).toEqual(expected)
  })

  test.prop([fc.string(), fc.string()])('reconstructs both inputs from prefix and cores', (from, to) => {
    const { fromCore, prefix, toCore } = editPlan(from, to)
    expect(prefix + fromCore).toBe(from)
    expect(prefix + toCore).toBe(to)
  })
})

const entries: readonly LandingTypingDemoEntry[] = [
  { anchor: 'a = ', from: 'false', kind: 'edit', slug: 'a', to: 'true' },
  { anchor: 'b = ', from: '0',     kind: 'edit', slug: 'b', to: '1' }
]

describe('applyCompletedEdits', () => {
  it('applies edits up to the cursor, leaving later ones untouched', () => {
    expect(applyCompletedEdits('a = false\nb = 0\n', entries, 1)).toBe('a = true\nb = 0\n')
    expect(applyCompletedEdits('a = false\nb = 0\n', entries, 2)).toBe('a = true\nb = 1\n')
  })

  it('skips an edit whose anchor is absent', () => {
    expect(applyCompletedEdits('unrelated', entries, 2)).toBe('unrelated')
  })
})

const entry: LandingTypingDemoEntry = {
  anchor: 'x = ', from: 'false', kind: 'edit', slug: 'x', to: 'true'
}

describe('segmentsForEdit', () => {
  it.each([
    ['editBackspacing', 2, 'fal'],
    ['editTyping',      2, 'tr'],
    ['holdAfterTyped',  0, 'true']
  ])('renders the editing segment for %s', (phase, progress, editing) => {
    expect(segmentsForEdit(entry, 'before\nx = false\nafter', phase, progress).editing)
      .toBe(editing)
  })

  it('splits the surrounding lines into context', () => {
    const seg = segmentsForEdit(entry, 'a\nx = false\nb', 'editTyping', 0)
    expect({
      after             : seg.after,
      before            : seg.before,
      editingLineAfter  : seg.editingLineAfter,
      editingLineBefore : seg.editingLineBefore
    }).toEqual({ after: '\nb', before: 'a\n', editingLineAfter: '', editingLineBefore: 'x = ' })
  })

  it('handles an edit with no surrounding newlines', () => {
    expect(segmentsForEdit(entry, 'x = false', 'editTyping', 0)).toEqual({
      after: '', before: '', editing: '', editingLineAfter: '', editingLineBefore: 'x = '
    })
  })

  it('returns the whole text as before when the anchor is absent', () => {
    expect(segmentsForEdit(entry, 'no anchor', 'editTyping', 0))
      .toEqual({ ...EMPTY_SEGMENTS, before: 'no anchor' })
  })
})

const rows: readonly LandingTypingDemoResetRow[] = [
  { anchor: 'x = ', end: 'true', prelude: 'false' }
]

describe('resetText', () => {
  it.each([
    ['resetBackspacing', 1, 'x = tru\n'],
    ['resetTyping',      2, 'x = fa\n']
  ])('rewrites the row for %s', (phase, progress, expected) => {
    expect(resetText('x = false\n', rows, phase, progress)).toBe(expected)
  })

  it('skips a row whose anchor is absent', () => {
    expect(resetText('unrelated', rows, 'resetTyping', 1)).toBe('unrelated')
  })
})
