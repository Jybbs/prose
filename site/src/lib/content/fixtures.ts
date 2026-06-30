import { existsSync, readdirSync } from 'node:fs'
import fs                          from 'node:fs/promises'
import path                        from 'node:path'
import { fileURLToPath }           from 'node:url'

import { parse }       from 'smol-toml'
import type { Loader } from 'astro/loaders'

const FINDINGS_FILE = 'lint_findings.snap'
const INPUT_FILE    = 'input.py'
const META_FILE     = 'meta.toml'
const SNAPSHOT_FILE = 'input.py.snap'

interface FixtureDocs {
  canonical?   : boolean
  description?  : string
  previewable? : boolean
  title?        : string
}

// Folds a fixture case directory into one entry the built-in loaders cannot
// produce, pairing the input with the snapshot output, the lint findings the
// harness emits, and the `[docs]` table that surfaces the case on its rule
// page. The case id is `<rule>/<case>` with the rule slug in kebab form so it
// joins the docs collection's rule slugs.
export function fixturesLoader(): Loader {
  return {
    name: 'prose-fixtures',
    load: async ({ config, parseData, store }) => {
      const root = fileURLToPath(new URL('../crate/tests/fixtures/', config.root))
      store.clear()
      for (const rule of subdirectories(root)) {
        const ruleDir = path.join(root, rule)
        for (const name of subdirectories(ruleDir)) {
          const dir   = path.join(ruleDir, name)
          const input = path.join(dir, INPUT_FILE)
          const snap  = path.join(dir, SNAPSHOT_FILE)
          if (!existsSync(input) || !existsSync(snap)) continue

          const [source, snapshot] = await Promise.all([
            fs.readFile(input, 'utf8'),
            fs.readFile(snap, 'utf8')
          ])
          const id   = `${rule.replaceAll('_', '-')}/${name}`
          const data = await parseData({
            id,
            data: {
              findings : await readFindings(dir),
              input    : source,
              output   : snapshotBody(snapshot).replace(/\s+$/, '\n'),
              ...(await readDocs(dir))
            }
          })
          store.set({ data, id })
        }
      }
    }
  }
}

const subdirectories = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).filter(e => e.isDirectory()).map(e => e.name).sort()

// Drops the leading insta YAML frontmatter the snapshot tooling writes,
// leaving the recorded body the source-of-truth output.
function snapshotBody(raw: string): string {
  const close = raw.startsWith('---\n') ? raw.indexOf('\n---\n', 4) : -1
  return close === -1 ? raw : raw.slice(close + 5)
}

async function readFindings(dir: string): Promise<unknown[]> {
  const file = path.join(dir, FINDINGS_FILE)
  if (!existsSync(file)) return []
  const body = snapshotBody(await fs.readFile(file, 'utf8')).trim()
  return body ? (JSON.parse(body) as unknown[]) : []
}

async function readDocs(dir: string): Promise<FixtureDocs> {
  const file = path.join(dir, META_FILE)
  if (!existsSync(file)) return {}
  return (parse(await fs.readFile(file, 'utf8')) as { docs?: FixtureDocs }).docs ?? {}
}
