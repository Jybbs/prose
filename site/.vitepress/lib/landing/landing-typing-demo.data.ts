import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import type { KeyedTokensInfo } from '@shikijs/magic-move/types'
import { defineLoader }         from 'vitepress'

import * as typingDemo         from './typing-demo'
import { precompileMagicMove } from '../markdown/magic-move'
import * as paths              from '../shared/paths'

export type {
  LandingTypingDemoEditEntry,
  LandingTypingDemoEntry,
  LandingTypingDemoResetRow
} from './typing-demo'

interface LandingTypingDemoData {
  entries          : readonly typingDemo.LandingTypingDemoEntry[]
  prelude          : string
  pythonStateSteps : readonly KeyedTokensInfo[]
  resetRows        : readonly typingDemo.LandingTypingDemoResetRow[]
}

declare const data: LandingTypingDemoData
export { data }

const root = paths.repoRoot(import.meta.url)

export default defineLoader({
  watch: paths.proseBinaryCandidates(root),
  async load(): Promise<LandingTypingDemoData> {
    const states: string[] = [typingDemo.SOURCE]
    for (let i = 0; i < typingDemo.RULES.length; i++) {
      states.push(formatState(typingDemo.SOURCE, typingDemo.RULES.slice(0, i + 1).join(',')))
    }
    for (const entry of typingDemo.ENTRIES) {
      if (entry.tail !== undefined) {
        states.push(formatState(typingDemo.SOURCE, typingDemo.RULES.join(','), entry.tail))
      }
    }

    return {
      entries          : typingDemo.ENTRIES,
      prelude          : typingDemo.PRELUDE,
      pythonStateSteps : await precompileMagicMove(states),
      resetRows        : typingDemo.RESET_ROWS
    }
  }
})

function formatState(source: string, select: string, configToml?: string): string {
  const args = ['format', '--stdin', '--select', select]
  if (configToml === undefined) {
    return paths.runProse(root, args, { input: source, stdio: 'pipe' })
  }
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-demo-'))
  try {
    fs.writeFileSync(path.join(tmpDir, 'prose.toml'), configToml)
    return paths.runProse(root, args, { cwd: tmpDir, input: source, stdio: 'pipe' })
  } finally {
    fs.rmSync(tmpDir, { force: true, recursive: true })
  }
}
