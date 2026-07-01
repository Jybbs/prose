import { execFileSync }                       from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir }                             from 'node:os'
import path                                   from 'node:path'

import type { Loader } from 'astro/loaders'

import { ENTRIES, PRELUDE, RESET_ROWS, RULES, SOURCE } from '../landing/typing-demo'
import { precompileMagicMove }                         from '../markdown/magic-move'
import { resolveProseBinary }                          from '../shared/paths'
import { replaceStore }                                from './store'

// Runs the compiled `prose` binary once per morph state, the source first, then
// the cumulative rule selections, then each config-tail variant, and precompiles
// the magic-move token steps the landing typing demo animates between. Requires
// a prior `cargo build`.
export function typingDemoLoader(): Loader {
  return {
    name: 'prose-typing-demo',
    load: async ctx => {
      const binary = resolveProseBinary(ctx.config.root)

      const states = [
        SOURCE,
        ...RULES.map((_, index) => runProse(binary, SOURCE, RULES.slice(0, index + 1).join(','))),
        ...ENTRIES
          .filter(entry => entry.tail !== undefined)
          .map(entry => runProse(binary, SOURCE, RULES.join(','), entry.tail))
      ]

      await replaceStore(ctx, [{
        id   : 'landing',
        data : {
          entries          : ENTRIES,
          prelude          : PRELUDE,
          pythonStateSteps : await precompileMagicMove(states),
          resetRows        : RESET_ROWS
        }
      }])
    }
  }
}

function runProse(binary: string, source: string, select: string, configToml?: string): string {
  const args = ['format', '--stdin', '--select', select]
  const run  = (cwd?: string): string => execFileSync(binary, args, {
    cwd,
    encoding : 'utf8',
    input    : source,
    stdio    : ['pipe', 'pipe', 'pipe']
  })
  if (configToml === undefined) return run()
  const dir = mkdtempSync(path.join(tmpdir(), 'prose-demo-'))
  try {
    writeFileSync(path.join(dir, 'prose.toml'), configToml)
    return run(dir)
  } finally {
    rmSync(dir, { force: true, recursive: true })
  }
}
