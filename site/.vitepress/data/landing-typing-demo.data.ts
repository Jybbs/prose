import { execFileSync } from 'node:child_process'
import fs               from 'node:fs'
import os               from 'node:os'
import path             from 'node:path'

import type { KeyedTokensInfo } from 'shiki-magic-move/types'
import { defineLoader }         from 'vitepress'

import { ENTRIES, PRELUDE, RESET_ROWS, RULES, SOURCE } from '../theme/components/landing/typing-demo-fixtures'
import type {
  LandingTypingDemoEditEntry,
  LandingTypingDemoEntry,
  LandingTypingDemoResetRow
} from '../theme/components/landing/typing-demo-fixtures'
import { precompileMagicMove } from '../lib/markdown/magic-move'
import { repoRoot }            from '../lib/shared/paths'

export type {
  LandingTypingDemoEditEntry,
  LandingTypingDemoEntry,
  LandingTypingDemoResetRow
}

interface LandingTypingDemoData {
  entries          : readonly LandingTypingDemoEntry[]
  prelude          : string
  pythonStateSteps : readonly KeyedTokensInfo[]
  resetRows        : readonly LandingTypingDemoResetRow[]
}

declare const data: LandingTypingDemoData
export { data }

const root = repoRoot(import.meta.url)

export default defineLoader({
  watch: [
    path.join(root, 'target/release/prose'),
    path.join(root, 'target/debug/prose')
  ],
  async load(): Promise<LandingTypingDemoData> {
    const bin = proseBinary()

    const states: string[] = [SOURCE]
    for (let i = 0; i < RULES.length; i++) {
      states.push(runProse(bin, SOURCE, RULES.slice(0, i + 1).join(',')))
    }
    for (const entry of ENTRIES) {
      if (entry.tail !== undefined) {
        states.push(runProse(bin, SOURCE, RULES.join(','), entry.tail))
      }
    }

    return {
      entries          : ENTRIES,
      prelude          : PRELUDE,
      pythonStateSteps : await precompileMagicMove(states),
      resetRows        : RESET_ROWS
    }
  }
})

function proseBinary(): string {
  const found = ['target/release/prose', 'target/debug/prose']
    .map(p => path.join(root, p))
    .find(fs.existsSync)
  if (found) return found
  throw new Error('prose binary not found at target/{release,debug}/prose. Run `cargo build` first.')
}

function runProse(bin: string, source: string, select: string, configToml?: string): string {
  const args = ['format', '--stdin', '--select', select]
  if (configToml === undefined) {
    return execFileSync(bin, args, { encoding: 'utf8', input: source, stdio: ['pipe', 'pipe', 'pipe'] })
  }
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-demo-'))
  try {
    fs.writeFileSync(path.join(tmpDir, 'prose.toml'), configToml)
    return execFileSync(bin, args, { cwd: tmpDir, encoding: 'utf8', input: source, stdio: ['pipe', 'pipe', 'pipe'] })
  } finally {
    fs.rmSync(tmpDir, { force: true, recursive: true })
  }
}
