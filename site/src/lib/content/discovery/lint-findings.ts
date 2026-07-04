import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir }        from 'node:os'
import path              from 'node:path'
import { pathToFileURL } from 'node:url'

import { fixturesDir }                                         from '../../shared/paths'
import { FINDINGS_FILE, fixtureDirs, fixtureId, snapshotBody } from './fixtures-tree'
import type { LintFinding }                                    from '../schemas'

// The lint findings the decoration plugin draws, keyed by the `<rule>/<case>`
// fixture id a `lint=` fence names. Read from the harness snapshots at config
// load, before the fixtures collection exists, and holding only cases that
// carry findings.
export function discoverLintFindings(siteRoot: URL): Map<string, LintFinding[]> {
  const root = fixturesDir(siteRoot)
  const out  = new Map<string, LintFinding[]>()
  for (const { dir, name, rule } of fixtureDirs(root)) {
    const findings = readFindings(dir)
    if (findings.length > 0) out.set(fixtureId(rule, name), findings)
  }
  return out
}

export function readFindings(dir: string): LintFinding[] {
  const file = path.join(dir, FINDINGS_FILE)
  if (!existsSync(file)) return []
  const body = snapshotBody(readFileSync(file, 'utf8')).trim()
  return body ? (JSON.parse(body) as LintFinding[]) : []
}

if (import.meta.vitest) {
  const { afterAll, beforeAll, describe, expect, test } = import.meta.vitest

  const snap = (body: string): string =>
    `---\nsource: tests/diagnostics.rs\nexpression: json\ninput_file: x\n---\n${body}`
  const BASIC = snap('[\n  { "code": "align-equals", "fix": null, "message": "Alignment finding" }\n]\n')
  const GROUP = snap('[\n  { "code": "group-imports", "fix": { "applicability": "safe" }, "message": "Group finding" }\n]\n')

  let corpus  : string
  let fixtures: string
  let siteRoot: URL

  beforeAll(() => {
    corpus   = mkdtempSync(path.join(tmpdir(), 'lint-findings-'))
    fixtures = path.join(corpus, 'crate', 'tests', 'fixtures')
    const write = (rule: string, name: string, file: string, body: string): void => {
      const dir = path.join(fixtures, rule, name)
      mkdirSync(dir, { recursive: true })
      writeFileSync(path.join(dir, file), body)
    }
    write('align_equals',  'basic',   FINDINGS_FILE, BASIC)
    write('align_equals',  'blank',   FINDINGS_FILE, snap(''))
    write('align_equals',  'empty',   FINDINGS_FILE, snap('[]\n'))
    write('align_equals',  'no_snap', 'input.py',    'x = 1\n')
    write('group_imports', 'single',  FINDINGS_FILE, GROUP)
    siteRoot = pathToFileURL(path.join(corpus, 'site') + path.sep)
  })

  afterAll(() => rmSync(corpus, { force: true, recursive: true }))

  describe('fixtureDirs', () => {
    test('walks every case under every rule, sorted', () => {
      expect([...fixtureDirs(fixtures)].map(({ name, rule }) => ({ name, rule }))).toEqual([
        { name: 'basic',   rule: 'align_equals'  },
        { name: 'blank',   rule: 'align_equals'  },
        { name: 'empty',   rule: 'align_equals'  },
        { name: 'no_snap', rule: 'align_equals'  },
        { name: 'single',  rule: 'group_imports' }
      ])
    })
  })

  describe('readFindings', () => {
    test('parses a snapshot that carries findings', () => {
      const findings = readFindings(path.join(fixtures, 'align_equals', 'basic'))
      expect(findings).toHaveLength(1)
      expect(findings[0].code).toBe('align-equals')
      expect(findings[0].message).toBe('Alignment finding')
    })

    test.each(['empty', 'blank', 'no_snap'])('returns no findings for %s', name => {
      expect(readFindings(path.join(fixtures, 'align_equals', name))).toEqual([])
    })
  })

  describe('discoverLintFindings', () => {
    test('keys findings by fixture id, holding only cases that carry them', () => {
      const found = discoverLintFindings(siteRoot)
      expect([...found.keys()].sort()).toEqual(['align-equals/basic', 'group-imports/single'])
      expect(found.get('align-equals/basic')).toHaveLength(1)
      expect(found.get('group-imports/single')?.[0].fix?.applicability).toBe('safe')
    })
  })
}
