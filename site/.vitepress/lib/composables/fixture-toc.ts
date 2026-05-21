import { reactive } from 'vue'

export interface FixtureTocEntry {
  id    : string
  rule  : string
  title : string
}

const entries = reactive<FixtureTocEntry[]>([])

export function registerFixture(entry: FixtureTocEntry): () => void {
  entries.push(entry)
  return () => {
    const idx = entries.indexOf(entry)
    if (idx >= 0) entries.splice(idx, 1)
  }
}

export function fixtureTocFor(rule: string): readonly FixtureTocEntry[] {
  return entries.filter(e => e.rule === rule)
}
