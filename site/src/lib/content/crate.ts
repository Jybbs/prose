import { execFileSync } from 'node:child_process'
import fs               from 'node:fs/promises'

import { parse }       from 'smol-toml'
import type { Loader } from 'astro/loaders'

import { cargoTomlPath, resolveProseBinary } from '../shared/paths'

export type PipelineEntry = {
  imperative : string
  position   : number
  slug       : string
}

export function parseCrateVersion(toml: string, source: string): string {
  const version = (parse(toml) as { package?: { version?: unknown } }).package?.version
  if (typeof version !== 'string') throw new Error(`package.version not found in ${source}`)
  return version
}

// Loads the pipeline order as one entry per rule for the pipeline-order page,
// read from the `prose rules` JSON the binary emits so the registry in
// `crate/src/rule.rs` stays the single source of truth.
export function pipelineLoader(): Loader {
  return {
    name: 'prose-pipeline',
    load: async ({ config, parseData, store }) => {
      const binary  = resolveProseBinary(config.root)
      const json    = execFileSync(binary, ['rules', '--output-format', 'json'], { encoding: 'utf8' })
      const entries = JSON.parse(json) as PipelineEntry[]
      store.clear()
      for (const entry of entries) {
        store.set({ data: await parseData({ data: entry, id: entry.slug }), id: entry.slug })
      }
    }
  }
}

// Loads the crate version as a single entry the Open Graph enrichment reads.
export function releaseLoader(): Loader {
  return {
    name: 'prose-release',
    load: async ({ config, parseData, store }) => {
      const source  = cargoTomlPath(config.root)
      const version = parseCrateVersion(await fs.readFile(source, 'utf8'), source)
      store.clear()
      store.set({ data: await parseData({ data: { version }, id: 'release' }), id: 'release' })
    }
  }
}
