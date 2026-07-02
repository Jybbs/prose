import { atom } from 'nanostores'

// The folio state the glossary index and pane share, so a row click, a step
// button, and the search box all move the same selection. A `null` slug keeps
// the server-rendered default, the first entry in caseless slug order.
export const glossaryFolioQuery = atom<string>('')
export const glossaryFolioSlug  = atom<string | null>(null)
