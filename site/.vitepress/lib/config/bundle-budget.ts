import fs   from 'node:fs'
import path from 'node:path'

// Holds a byte count for each kind of file the budget measures.
interface Bundle {
  css : number
  js  : number
}

// Holds the bytes of CSS and JavaScript the built site may ship.
const BUNDLE_BUDGET: Bundle = { css: 314_368, js: 16_781_312 }

// Throws where the CSS or the JavaScript the build wrote under `outDir` measures
// more bytes than the budget holds, naming each kind that grew past it.
export function checkBundleBudget(outDir: string, budget: Bundle = BUNDLE_BUDGET): void {
  const measured = measureBundle(outDir)
  const over     = (Object.keys(budget) as (keyof Bundle)[]).filter(kind => measured[kind] > budget[kind])
  if (over.length > 0) {
    throw new Error(over
      .map(kind => `the built site's ${kind} measures ${measured[kind]} bytes against a budget of ${budget[kind]}`)
      .join(' and '))
  }
}

// Sums the bytes of the CSS and the JavaScript files under `outDir`.
export function measureBundle(outDir: string): Bundle {
  const bytes = (pattern: string) => fs.globSync(pattern, { cwd: outDir })
    .reduce((sum, file) => sum + fs.statSync(path.join(outDir, file)).size, 0)
  return { css: bytes('**/*.css'), js: bytes('**/*.js') }
}
