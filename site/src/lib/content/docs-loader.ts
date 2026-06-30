import { docsLoader }           from '@astrojs/starlight/loaders'
import type { DataStore, Loader } from 'astro/loaders'

import { assertCorpusIntegrity } from './integrity'
import type { CorpusEntry }      from './integrity'

const DOCS_ROOT = 'src/content/docs/'

// Wraps Starlight's `docsLoader` so the cross-record integrity pass runs once
// the store is populated, since a loader sees its own collection in full where
// a per-record schema sees one entry at a time.
export function docsLoaderWithIntegrity(): Loader {
  const inner = docsLoader()
  return {
    ...inner,
    name: 'docs-with-integrity',
    load: async context => {
      await inner.load(context)
      assertCorpusIntegrity(corpusEntries(context.store))
    }
  }
}

function* corpusEntries(store: DataStore): Generator<CorpusEntry> {
  for (const { data, filePath } of store.values()) {
    if (filePath === undefined) continue
    const root = filePath.indexOf(DOCS_ROOT)
    if (root !== -1) yield { data, path: filePath.slice(root + DOCS_ROOT.length) }
  }
}
