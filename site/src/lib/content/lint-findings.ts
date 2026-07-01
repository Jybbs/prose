import { existsSync, readFileSync } from 'node:fs'
import path                         from 'node:path'

import { fixturesDir }                          from '../shared/paths'
import { fixtureDirs, fixtureId, snapshotBody } from './fixtures-tree'
import type { LintFinding }                     from './schemas'

const FINDINGS_FILE = 'lint_findings.snap'

// The lint findings the decoration plugin draws, keyed by the `<rule>/<case>`
// fixture id a `lint=` fence names. Read from the harness snapshots at config
// load, before the fixtures collection exists, and holding only cases that
// carry findings.
export function discoverLintFindings(siteRoot: URL): Map<string, LintFinding[]> {
  const root = fixturesDir(siteRoot)
  const out  = new Map<string, LintFinding[]>()
  for (const { dir, name, rule } of fixtureDirs(root)) {
    const findings = readFindings(dir)
    if (findings.length) out.set(fixtureId(rule, name), findings)
  }
  return out
}

export function readFindings(dir: string): LintFinding[] {
  const file = path.join(dir, FINDINGS_FILE)
  if (!existsSync(file)) return []
  const body = snapshotBody(readFileSync(file, 'utf8')).trim()
  return body ? (JSON.parse(body) as LintFinding[]) : []
}
