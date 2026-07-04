// The docs collection strips a trailing `/index` from each slug-joined id, so a
// family index is `rules/<family>`, a rule leaf is `rules/<family>/<slug>`, and
// a flat-section leaf is `<section>/<slug>`. Each reader returns `undefined` for
// foreign sections and the one-segment section indexes.

interface FamilyPage {
  family : string
  slug   : string
}

export const familyIndex = (id: string): string | undefined => sectionLeaf(id, 'rules')

export function familyPage(id: string): FamilyPage | undefined {
  const segments = id.split('/')
  if (segments.length !== 3 || segments[0] !== 'rules') return undefined
  return { family: segments[1], slug: segments[2] }
}

export function sectionLeaf(id: string, section: string): string | undefined {
  const segments = id.split('/')
  if (segments.length !== 2 || segments[0] !== section) return undefined
  return segments[1]
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('familyIndex', () => {
    test.each([
      { name: 'reads the family off a rules index', id: 'rules/alignment',              expected: 'alignment' },
      { name: 'rejects a rule leaf page',           id: 'rules/alignment/align-equals', expected: undefined },
      { name: 'rejects the rules section index',    id: 'rules',                        expected: undefined },
      { name: 'rejects a foreign section leaf',     id: 'primitives/edge-magnet',       expected: undefined }
    ])('$name', ({ expected, id }) => {
      expect(familyIndex(id)).toEqual(expected)
    })
  })

  describe('familyPage', () => {
    test.each([
      { name: 'reads family and slug off a rule page', id: 'rules/alignment/align-equals', expected: { family: 'alignment', slug: 'align-equals' } },
      { name: 'rejects the family index',              id: 'rules/alignment',              expected: undefined },
      { name: 'rejects a flat-section leaf',           id: 'primitives/foo',               expected: undefined },
      { name: 'rejects a four-segment id',             id: 'rules/a/b/c',                  expected: undefined }
    ])('$name', ({ expected, id }) => {
      expect(familyPage(id)).toEqual(expected)
    })
  })

  describe('sectionLeaf', () => {
    test.each([
      { name: 'reads the leaf slug in-section', id: 'primitives/edge-magnet', expected: 'edge-magnet' },
      { name: 'rejects the section index',      id: 'primitives',             expected: undefined },
      { name: 'rejects a foreign section',      id: 'rules/edge-magnet',      expected: undefined },
      { name: 'rejects a nested id',            id: 'primitives/a/b',         expected: undefined }
    ])('$name', ({ expected, id }) => {
      expect(sectionLeaf(id, 'primitives')).toEqual(expected)
    })
  })
}
