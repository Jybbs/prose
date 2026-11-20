import { execFileSync } from 'node:child_process'
import fs               from 'node:fs'
import path             from 'node:path'

import { babelParse, walk } from '@vue/compiler-sfc'

import * as paths         from '../../lib/shared/paths.ts'
import { supportTest }    from '../support.ts'
import * as tailwind      from '../tailwind.ts'
import { CRATE, READERS } from '../../vitest.config.ts'

const FOLDER   = /^(?=.*\/).*[^./]/
const GLOBS    : Readonly<Record<string, readonly string[]>> = READERS
const ROOT     = paths.repoRoot(import.meta.url)
const RUNNER   = /\b(runProse|readProseUnder)\(/
const SCRIPT   = /\.(?:tsx?|vue|\{[\w,]*\b(?:tsx?|vue)\b[\w,]*\})$/
const SITE     = 'site/.vitepress'
const CONFIGS  = new Set([`${SITE}/knip.ts`, `${SITE}/vitest.config.ts`])
const TAILWIND = `${SITE}/tests/tailwind.ts`
const LISTING  = ['ls-files', '-z', '--', '*.mjs', '*.ts', '*.tsx', '*.vue']
const TRACKED  = execFileSync('git', LISTING, { cwd: ROOT, encoding: 'utf8' })
  .split('\0')
  .filter(file => file && fs.existsSync(path.join(ROOT, file)))
const MODULES  = TRACKED
  .filter(file => path.matchesGlob(file, `${SITE}/**/*.{mjs,ts,tsx}`) && !CONFIGS.has(file))
  .map(file => ({ file, text: fs.readFileSync(path.join(ROOT, file), 'utf8') }))
const READS = MODULES
  .map(({ file, text }) => [file, scriptsRead(file, text)] as const)
  .filter(([, read]) => read.length > 0)
const RUNNERS = MODULES
  .filter(({ file }) => file.startsWith(`${SITE}/lib/`) && !file.endsWith('/paths.ts'))
  .filter(({ text }) => RUNNER.test(text))
  .map(({ file }) => file)

// Names each string a module writes and each template literal, cut after the
// last slash ahead of its first expression, leaving out the specifiers of its
// imports and of `import.meta.glob`, which the module graph follows itself, and
// the keys and string method arguments that name no file.
function namedPaths(text: string): string[] {
  const named = [] as string[]
  const keys  = new WeakSet<object>()
  walk(babelParse(text, { plugins: ['jsx', 'typescript'], sourceType: 'module' }) as never, {
    enter(raw: unknown) {
      const node   = raw as Record<string, any>
      const callee = node.type === 'CallExpression' ? node.callee : undefined
      if (keys.has(node) || node.source || node.type === 'TSImportType') return this.skip()
      if (callee?.type === 'Import' || callee?.object?.type === 'MetaProperty') return this.skip()
      if (node.type === 'MemberExpression' && node.computed) keys.add(node.property)
      if (callee?.object?.name !== 'path' && Object.hasOwn(String.prototype, callee?.property?.name)) {
        node.arguments.forEach(keys.add, keys)
      }
      if (node.type === 'StringLiteral') named.push(node.value)
      if (node.type !== 'TemplateLiteral') return
      const prefix: string = node.quasis[0].value.cooked
      named.push(node.expressions.length > 0 ? prefix.slice(0, prefix.lastIndexOf('/') + 1) : prefix)
    }
  })
  return named
}

// Lists each tracked script a path `file` names matches, or holds where the
// path has a slash and a folder name, read from the repository, the site, its
// `.vitepress` folder, or the folder holding `file`, and adding the files the
// `@source` lines of the Tailwind entry name where `file` is the module
// scanning them.
function scriptsRead(file: string, text: string): string[] {
  const bases  = ['', 'site', SITE, path.dirname(file)]
  const named  = namedPaths(text)
  const join   = (names: string[]) => names.flatMap(name => bases.map(base => path.posix.join(base, name)))
  const globs  = join(named.filter(name => SCRIPT.test(name)))
  const dirs   = join(named.filter(name => FOLDER.test(name)).map(name => `${name}/`))
  const listed = TRACKED.filter(script =>
    globs.some(glob => path.matchesGlob(script, glob)) || dirs.some(dir => script.startsWith(dir))
  )
  return file === TAILWIND ? [...listed, ...tailwind.sourceFiles().map(name => `site/${name}`)] : listed
}

describe('READERS', () => {
  supportTest.for(RUNNERS)('reads the crate sources for %s, which runs the binary', file => {
    expect(READERS).toHaveProperty([file], expect.arrayContaining([CRATE]))
  })

  it('scans each `.mjs`, `.ts`, and `.tsx` module', () => {
    expect(new Set(MODULES.map(({ file }) => path.extname(file)))).toStrictEqual(new Set(['.mjs', '.ts', '.tsx']))
  })

  supportTest.for(READS)('lists a glob for each script %s reads', ([file, read]) => {
    const covered = new Set(fs.globSync(GLOBS[file] ?? [], { cwd: ROOT }))
    expect([...new Set(read).difference(covered)]).toStrictEqual([])
  })
})

describe('scriptsRead', () => {
  const probe = `${SITE}/tests/shared/probe.ts`

  it.each([
    ['an import',                "import { read } from '../../tests/vocabulary.ts'"],
    ['an `import.meta.glob`',    "export const sheets = import.meta.glob('../tailwind.ts')"],
    ['a computed key',           "export const pick = table[`../../theme/${name}.tsx`]"],
    ['a string method argument', "export const kit = name.startsWith('../../kit/')"],
    ['a type import',            "export type Theme = typeof import('../../theme/index.ts')"]
  ])('reads no script %s names', (_, text) => {
    expect(scriptsRead(probe, text)).toStrictEqual([])
  })

  it.each([
    ['the repository',   "read('site/.vitepress/theme/index.ts')"],
    ['the site',         "read('.vitepress/theme/index.ts')"],
    ['`.vitepress`',     "read('theme/index.ts')"],
    ['the module',       "read('../../theme/index.ts')"],
    ['`path.normalize`', "read(path.normalize('../../theme/index.ts'))"]
  ])('reads a script path from %s', (_, text) => {
    expect(scriptsRead(probe, text)).toStrictEqual([`${SITE}/theme/index.ts`])
  })

  it.each([
    ['a folder',                                "read('../../lib/og/render')"],
    ['the static prefix of a template literal', "read(`../../lib/og/render/${name}`)"],
    ['a prefix ending inside a file name',      "read(`../../lib/og/render/bu${name}`)"],
    ['a prefix holding one folder',             "read(`lib/${name}`)"]
  ])('reads the scripts under %s', (_, text) => {
    expect(scriptsRead(probe, text)).toContain(`${SITE}/lib/og/render/build.ts`)
  })

  it('reads a template literal holding no expression as the path it spells', () => {
    expect(scriptsRead(probe, 'read(`../../theme/notes.md`)')).toStrictEqual([])
  })

  it('adds the `@source` files to the Tailwind entry', () => {
    expect(scriptsRead(TAILWIND, '')).toStrictEqual(tailwind.sourceFiles().map(name => `site/${name}`))
  })
})
