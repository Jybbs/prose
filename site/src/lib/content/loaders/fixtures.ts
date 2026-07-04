import { existsSync } from 'node:fs'
import fs             from 'node:fs/promises'
import path           from 'node:path'

import { parse }       from 'smol-toml'
import type { Loader } from 'astro/loaders'

import { precompileMagicMove }           from '../../markdown/magic-move'
import { fixturesDir }                   from '../../shared/paths'
import {
  fixtureDirs, fixtureId, INPUT_FILE, META_FILE, SNAPSHOT_FILE, snapshotBody
} from '../discovery/fixtures-tree'
import { readFindings }                  from '../discovery/lint-findings'
import { replaceStore, type StoreEntry } from './store'

// A case earns the before/after toggle when its output differs from its input
// or lint findings decorate the after pane.
export const fixtureHasToggle = (
  data: { findings: readonly unknown[], input: string, output: string }
): boolean =>
  data.input !== data.output || data.findings.length > 0

// Folds a fixture case directory into one entry the built-in loaders cannot
// produce, pairing the input with the snapshot output, the lint findings the
// harness emits, and the `[docs]` table that surfaces the case on its rule
// page.
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

async function readDocs(dir: string): Promise<Record<string, unknown>> {
  const file = path.join(dir, META_FILE)
  if (!existsSync(file)) return {}
  const raw = await fs.readFile(file, 'utf8')
  return (parse(raw) as { docs?: Record<string, unknown> }).docs ?? {}
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

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('fixtureHasToggle', () => {
    test.each([
      { data: { findings: [],   input: 'a', output: 'b' }, name: 'output differs',    want: true },
      { data: { findings: [{}], input: 'a', output: 'a' }, name: 'findings decorate',  want: true },
      { data: { findings: [],   input: 'a', output: 'a' }, name: 'identical, no lint', want: false }
    ])('$name -> $want', ({ data, want }) => {
      expect(fixtureHasToggle(data)).toBe(want)
    })
  })
}
