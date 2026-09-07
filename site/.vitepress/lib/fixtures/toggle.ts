import fs from 'node:fs/promises'

import matter from 'gray-matter'

import type { FixtureFlags } from './entry'
import { readLintFindings }  from './lint-findings'
import { snapshotPath }      from './walker'

interface FixtureToggleState extends FixtureFlags {
  inputRaw : string
  output   : string
}

// Reads a fixture's input, its snapshot, and its lint findings, and derives
// whether the card shows a before-and-after toggle, which the rule-fixture
// loader and the page renderer both read. The snapshot drops its insta
// frontmatter and normalizes trailing whitespace before the byte comparison
// against the input.
export async function readFixtureToggle(inputPath: string): Promise<FixtureToggleState> {
  const [inputRaw, snapRaw] = await Promise.all([
    fs.readFile(inputPath,               'utf8'),
    fs.readFile(snapshotPath(inputPath), 'utf8')
  ])
  const output        = matter(snapRaw).content.trimEnd() + '\n'
  const changesSource = inputRaw !== output
  const hasFindings   = readLintFindings(inputPath).length > 0
  return {
    changesSource,
    hasFindings,
    hasToggle: changesSource || hasFindings,
    inputRaw,
    output
  }
}
