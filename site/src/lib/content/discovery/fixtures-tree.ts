import path from 'node:path'

import { parseFrontmatter } from '@astrojs/markdown-remark'

import { subdirectories } from './page'

export const FINDINGS_FILE = 'lint_findings.snap'
export const INPUT_FILE    = 'input.py'
export const META_FILE     = 'meta.toml'
export const SNAPSHOT_FILE = 'input.py.snap'

// The `<rule>/<case>` id, the rule slug in kebab form so it joins the docs
// collection's rule slugs.
export const fixtureId = (rule: string, name: string): string => `${rule.replaceAll('_', '-')}/${name}`

// Each `<rule>/<case>` case directory under the fixtures root, the rule and case
// names alongside the joined path.
export function* fixtureDirs(root: string): Iterable<{ dir: string, name: string, rule: string }> {
  for (const rule of subdirectories(root)) {
    const ruleDir = path.join(root, rule)
    for (const name of subdirectories(ruleDir)) yield { dir: path.join(ruleDir, name), name, rule }
  }
}

// Drops the insta YAML frontmatter the snapshot tooling writes, leaving the
// recorded body the source-of-truth output. `parseFrontmatter` removes the
// frontmatter block, and the slice drops the newline it leaves before the body.
export function snapshotBody(raw: string): string {
  const content = parseFrontmatter(raw).content
  return content.startsWith('\n') ? content.slice(1) : content
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('constants', () => {
    test('name the harness files', () => {
      expect({ FINDINGS_FILE, INPUT_FILE, META_FILE, SNAPSHOT_FILE }).toEqual({
        FINDINGS_FILE : 'lint_findings.snap',
        INPUT_FILE    : 'input.py',
        META_FILE     : 'meta.toml',
        SNAPSHOT_FILE : 'input.py.snap'
      })
    })
  })

  describe('fixtureId', () => {
    test.each([
      { name: 'kebabs the rule and joins the case', rule: 'align_equals', case: 'basic', expected: 'align-equals/basic' },
      { name: 'kebabs every underscore',            rule: 'a_b_c',        case: 'x',     expected: 'a-b-c/x' }
    ])('$name', ({ case: name, expected, rule }) => {
      expect(fixtureId(rule, name)).toBe(expected)
    })
  })

  describe('snapshotBody', () => {
    test('drops the insta frontmatter and its leading newline', () => {
      const body = snapshotBody('---\nsource: tests/diagnostics.rs\nexpression: json\n---\n[1]\n')
      expect(body.startsWith('[')).toBe(true)
      expect(JSON.parse(body.trim())).toEqual([1])
    })

    test('returns a body carrying no frontmatter unchanged', () => {
      expect(snapshotBody('plain output\n')).toBe('plain output\n')
    })
  })
}
