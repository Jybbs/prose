import { atom } from 'nanostores'

// The folio state the glossary index and pane share, so a row click, a step
// button, and the search box all move the same selection. A `null` slug keeps
// the server-rendered default, the first entry in caseless slug order.
export const glossaryFolioQuery = atom<string>('')
export const glossaryFolioSlug  = atom<string | null>(null)

if (import.meta.vitest) {
  const { afterEach, describe, expect, test, vi } = import.meta.vitest

  afterEach(() => {
    glossaryFolioQuery.set('')
    glossaryFolioSlug.set(null)
  })

  describe('glossaryFolioQuery', () => {
    test('starts empty and reflects a write', () => {
      expect(glossaryFolioQuery.get()).toBe('')
      glossaryFolioQuery.set('align')
      expect(glossaryFolioQuery.get()).toBe('align')
    })

    test('notifies a subscriber on change', () => {
      const seen = vi.fn()
      const off  = glossaryFolioQuery.subscribe(seen)
      glossaryFolioQuery.set('equals')
      off()
      expect(seen.mock.lastCall?.[0]).toBe('equals')
    })
  })

  describe('glossaryFolioSlug', () => {
    test('starts null and reflects a selection', () => {
      expect(glossaryFolioSlug.get()).toBeNull()
      glossaryFolioSlug.set('align-equals')
      expect(glossaryFolioSlug.get()).toBe('align-equals')
    })
  })
}
