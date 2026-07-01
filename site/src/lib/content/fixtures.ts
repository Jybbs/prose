import { existsSync } from 'node:fs'
import fs             from 'node:fs/promises'
import path           from 'node:path'

import { parse }       from 'smol-toml'
import type { Loader } from 'astro/loaders'

import { precompileMagicMove }                  from '../markdown/magic-move'
import { fixturesDir }                          from '../shared/paths'
import { fixtureDirs, fixtureId, snapshotBody } from './fixtures-tree'
import { readFindings }                         from './lint-findings'
import { replaceStore, type StoreEntry }        from './store'

const INPUT_FILE    = 'input.py'
const META_FILE     = 'meta.toml'
const SNAPSHOT_FILE = 'input.py.snap'

// Folds a fixture case directory into one entry the built-in loaders cannot
// produce, pairing the input with the snapshot output, the lint findings the
// harness emits, and the `[docs]` table that surfaces the case on its rule
// page. The case id is `<rule>/<case>` with the rule slug in kebab form so it
// joins the docs collection's rule slugs.
export function fixturesLoader(): Loader {
  return {
    name: 'prose-fixtures',
    load: async ctx => {
      const root = fixturesDir(ctx.config.root)
      const entries: StoreEntry[] = []
      for (const { dir, name, rule } of fixtureDirs(root)) {
        const input = path.join(dir, INPUT_FILE)
        const snap  = path.join(dir, SNAPSHOT_FILE)
        if (!existsSync(input) || !existsSync(snap)) continue

        const [source, snapshot] = await Promise.all([
          fs.readFile(input, 'utf8'),
          fs.readFile(snap, 'utf8')
        ])
        const docs   = await readDocs(dir)
        const output = snapshotBody(snapshot).trimEnd() + '\n'
        entries.push({
          id   : fixtureId(rule, name),
          data : {
            findings : readFindings(dir),
            input    : source,
            output,
            ...docs,
            ...(await previewSteps(docs, source, output))
          }
        })
      }
      await replaceStore(ctx, entries)
    }
  }
}

async function readOptional(dir: string, name: string): Promise<string | null> {
  const file = path.join(dir, name)
  return existsSync(file) ? fs.readFile(file, 'utf8') : null
}

async function readDocs(dir: string): Promise<Record<string, unknown>> {
  const raw = await readOptional(dir, META_FILE)
  return raw === null ? {} : (parse(raw) as { docs?: Record<string, unknown> }).docs ?? {}
}

// Precompiles the before/after magic-move token steps for a previewable case
// whose output differs from its input, leaving the identical-state case with no
// steps so the downstream pane renders a single static block.
async function previewSteps(
  docs   : Record<string, unknown>,
  input  : string,
  output : string
): Promise<Record<string, unknown>> {
  if (docs.previewable !== true || input === output) return {}
  return { steps: await precompileMagicMove([input, output]) }
}
