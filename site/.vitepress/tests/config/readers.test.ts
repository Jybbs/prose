import fs   from 'node:fs'
import path from 'node:path'

import { babelParse, walk } from '@vue/compiler-sfc'

import * as paths         from '../../lib/shared/paths.ts'
import { supportTest }    from '../support.ts'
import * as tailwind      from '../tailwind.ts'
import { CRATE, READERS } from '../../vitest.config.ts'

const GLOBS    : Readonly<Record<string, readonly string[]>> = READERS
const ROOT     = paths.repoRoot(import.meta.url)
const RUNNER   = /\b(runProse|readProseUnder)\(/
const SCRIPT   = /\.(?:tsx?|vue|\{[\w,]*\b(?:tsx?|vue)\b[\w,]*\})$/
const SITE     = 'site/.vitepress'
const CONFIGS  = [`${SITE}/knip.ts`, `${SITE}/vitest.config.ts`]
const TAILWIND = `${SITE}/tests/tailwind.ts`
const MODULES  = fs.globSync(`${SITE}/**/*.{mjs,ts,tsx}`, { cwd: ROOT, exclude: CONFIGS })
  .toSorted()
  .map(file => ({ file, text: fs.readFileSync(path.join(ROOT, file), 'utf8') }))
const READS = MODULES
  .map(({ file, text }) => [file, scriptsRead(file, text)] as const)
  .filter(([, read]) => read.length > 0)
const RUNNERS = MODULES
  .filter(({ file }) => file.startsWith(`${SITE}/lib/`) && !file.endsWith('/paths.ts'))
  .filter(({ text }) => RUNNER.test(text))
  .map(({ file }) => file)

// Names each string a module writes that ends on a script extension, leaving
// out the specifiers of its imports and of `import.meta.glob`, which the module
// graph follows itself.
function scriptPaths(text: string): string[] {
  const named = [] as string[]
  walk(babelParse(text, { plugins: ['jsx', 'typescript'], sourceType: 'module' }) as never, {
    enter(raw: unknown) {
      const node   = raw as Record<string, any>
      const callee = node.type === 'CallExpression' ? node.callee : undefined
      if (node.source || node.type === 'TSImportType') return this.skip()
      if (callee?.type === 'Import' || callee?.object?.type === 'MetaProperty') return this.skip()
      if (node.type === 'StringLiteral' && SCRIPT.test(node.value)) named.push(node.value)
    }
  })
  return named
}

// Lists each file under the repository a string of `file` matches, read as a
// path from the repository, the site, its `.vitepress` folder, or the folder
// holding `file`, and adding the files the `@source` lines of the Tailwind
// entry name where `file` is the module scanning them.
function scriptsRead(file: string, text: string): string[] {
  const bases  = ['', 'site', SITE, path.dirname(file)]
  const globs  = scriptPaths(text)
    .filter(name => !path.isAbsolute(name))
    .flatMap(name => bases.map(base => path.posix.join(base, name)))
  const listed = fs.globSync(globs, { cwd: ROOT, exclude: ['**/node_modules/**'] })
  return file === TAILWIND ? [...listed, ...tailwind.sourceFiles().map(name => `site/${name}`)] : listed
}

describe('READERS', () => {
  supportTest.for(RUNNERS)('reads the crate sources for %s, which runs the binary', file => {
    expect(READERS).toHaveProperty([file], expect.arrayContaining([CRATE]))
  })

  supportTest.for(READS)('lists a glob for each script %s reads', ([file, read]) => {
    const covered = new Set(fs.globSync(GLOBS[file] ?? [], { cwd: ROOT }))
    expect([...new Set(read).difference(covered)]).toStrictEqual([])
  })
})
