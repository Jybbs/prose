import fs from 'node:fs/promises'

import matter from 'gray-matter'

import { readLintFindings, type LintFinding } from './lint-findings'

interface FixtureToggleState {
  changesSource : boolean
  findings      : LintFinding[]
  hasFindings   : boolean
  hasToggle     : boolean
  inputRaw      : string
  output        : string
}

// Reads a fixture's input/snapshot pair and lint findings, deriving the
// `BEFORE|AFTER` toggle signal both fixture loaders key on. A card carries a
// toggle when the rule rewrites the source or flags it, and is otherwise a
// no-op marker. The snapshot strips its insta frontmatter and normalizes
// trailing whitespace before the byte compare against the input.
export async function readFixtureToggle(inputPath: string): Promise<FixtureToggleState> {
  const [inputRaw, snapRaw] = await Promise.all([
    fs.readFile(inputPath,           'utf8'),
    fs.readFile(`${inputPath}.snap`, 'utf8')
  ])
  const output        = matter(snapRaw).content.replace(/\s+$/, '\n')
  const findings      = readLintFindings(inputPath)
  const changesSource = inputRaw !== output
  const hasFindings   = findings.length > 0
  return {
    changesSource,
    findings,
    hasFindings,
    hasToggle: changesSource || hasFindings,
    inputRaw,
    output
  }
}
