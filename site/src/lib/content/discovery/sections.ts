// The docs collection's ids are slash-joined paths, so the discovery surfaces
// read section membership as segment structure rather than regex lookaheads.
// Each reader returns `undefined` for foreign sections and section indexes.

interface FamilyPage {
  family : string
  slug   : string
}

export function familyIndex(id: string): string | undefined {
  const segments = id.split('/')
  if (segments.length !== 3 || segments[0] !== 'rules' || segments[2] !== 'index') return undefined
  return segments[1]
}

export function familyPage(id: string): FamilyPage | undefined {
  const segments = id.split('/')
  if (segments.length !== 3 || segments[0] !== 'rules' || segments[2] === 'index') return undefined
  return { family: segments[1], slug: segments[2] }
}

export function sectionLeaf(id: string, section: string): string | undefined {
  const segments = id.split('/')
  if (segments.length !== 2 || segments[0] !== section || segments[1] === 'index') return undefined
  return segments[1]
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('familyIndex', () => {
    test.each([
      { name: 'reads the family off a rules index',   id: 'rules/alignment/index',       expected: 'alignment' },
      { name: 'rejects a rule leaf page',             id: 'rules/alignment/align-equals', expected: undefined },
      { name: 'rejects a two-segment id',             id: 'primitives/index',             expected: undefined },
      { name: 'rejects a four-segment id',            id: 'rules/a/b/c',                  expected: undefined },
      { name: 'rejects a foreign top segment',        id: 'guides/alignment/index',       expected: undefined }
    ])('$name', ({ expected, id }) => {
      expect(familyIndex(id)).toEqual(expected)
    })
  })

  describe('familyPage', () => {
    test.each([
      { name: 'reads family and slug off a rule page', id: 'rules/alignment/align-equals', expected: { family: 'alignment', slug: 'align-equals' } },
      { name: 'rejects the family index',              id: 'rules/alignment/index',        expected: undefined },
      { name: 'rejects a two-segment id',              id: 'primitives/foo',               expected: undefined },
      { name: 'rejects a four-segment id',             id: 'rules/a/b/c',                  expected: undefined }
    ])('$name', ({ expected, id }) => {
      expect(familyPage(id)).toEqual(expected)
    })
  })

  describe('sectionLeaf', () => {
    test.each([
      { name: 'reads the leaf slug in-section', id: 'primitives/edge-magnet', expected: 'edge-magnet' },
      { name: 'rejects the section index',      id: 'primitives/index',       expected: undefined },
      { name: 'rejects a bare section id',      id: 'primitives',             expected: undefined },
      { name: 'rejects a foreign section',      id: 'rules/edge-magnet',      expected: undefined },
      { name: 'rejects a nested id',            id: 'primitives/a/b',         expected: undefined }
    ])('$name', ({ expected, id }) => {
      expect(sectionLeaf(id, 'primitives')).toEqual(expected)
    })
  })
}
