import { execFileSync } from 'node:child_process'
import fs               from 'node:fs'
import os               from 'node:os'
import path             from 'node:path'

import { getSingletonHighlighter }                   from 'shiki'
import { codeToKeyedTokens, createMagicMoveMachine } from 'shiki-magic-move/core'
import type { KeyedTokensInfo }                      from 'shiki-magic-move/types'
import { defineLoader }                              from 'vitepress'

import { ENTRIES, PRELUDE, RULES, SOURCE }                                                 from './landing-typing-demo.fixtures'
import type { LandingTypingDemoAppendEntry, LandingTypingDemoEditEntry, LandingTypingDemoEntry } from './landing-typing-demo.fixtures'
import { SHIKI_THEMES }                                                                    from '../lib/shared/constants'
import { repoRoot }                                                                        from '../lib/shared/paths'

export type {
  LandingTypingDemoAppendEntry,
  LandingTypingDemoEditEntry,
  LandingTypingDemoEntry
}

export interface LandingTypingDemoData {
  entries          : readonly LandingTypingDemoEntry[]
  prelude          : string
  pythonStateSteps : readonly KeyedTokensInfo[]
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
      if (entry.kind === 'edit') {
        states.push(runProse(bin, SOURCE, RULES.join(','), entry.tail))
      }
    }

    const highlighter = await getSingletonHighlighter({
      langs  : ['python'],
      themes : Object.values(SHIKI_THEMES)
    })

    const machine = createMagicMoveMachine(code =>
      codeToKeyedTokens(highlighter, code, { lang: 'python', themes: SHIKI_THEMES })
    )
    const pythonStateSteps: KeyedTokensInfo[] = []
    for (const state of states) {
      const { current } = machine.commit(state)
      pythonStateSteps.push(current)
    }

    return {
      entries : ENTRIES,
      prelude : PRELUDE,
      pythonStateSteps
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
    return execFileSync(bin, args, { encoding: 'utf8', input: source })
  }
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-demo-'))
  try {
    fs.writeFileSync(path.join(tmpDir, 'pyproject.toml'), configToml)
    return execFileSync(bin, args, { cwd: tmpDir, encoding: 'utf8', input: source })
  } finally {
    fs.rmSync(tmpDir, { force: true, recursive: true })
  }
}
