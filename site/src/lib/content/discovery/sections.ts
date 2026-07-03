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
