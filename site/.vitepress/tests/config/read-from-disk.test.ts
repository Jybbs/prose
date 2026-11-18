import fs   from 'node:fs'
import path from 'node:path'

import { readFromDiskPlugin } from '../../lib/config/read-from-disk'
import { supportTest }        from '../support'

type CacheKey  = (context: { id: string }) => string | undefined
type Configure = (context: { defineCacheKeyGenerator: (generator: CacheKey) => void }) => void
type Transform = (this: { addWatchFile: (file: string) => void }, code: string, id: string) => void

const FILES = [
  'crate/src/lib.rs', 'crate/src/rules/a/mod.rs', 'crate/src/rules/b/lint.rs', 'site/lib/discovery.ts',
  'site/lib/other.ts', 'site/pages/guide.md'
]

const READERS = { 'site/lib/discovery.ts': ['crate/src/rules/**/*.rs', 'site/pages/**/*.md'] }

function tree(root: string): string {
  for (const file of FILES) {
    fs.mkdirSync(path.join(root, path.dirname(file)), { recursive: true })
    fs.writeFileSync(path.join(root, file), '')
  }
  return root
}

function watched(root: string, module: string): string[] {
  const files  = [] as string[]
  const plugin = readFromDiskPlugin(READERS, root)
  const hook   = plugin.transform as Transform
  hook.call({ addWatchFile: file => files.push(file) }, '', path.join(root, module))
  return files.toSorted()
}

function cacheKey(root: string, module: string): string | undefined {
  const generators = [] as CacheKey[]
  const plugin     = readFromDiskPlugin(READERS, root)
  const configure  = plugin.configureVitest as Configure
  configure({ defineCacheKeyGenerator: generator => generators.push(generator) })
  return generators[0]?.({ id: path.join(root, module) })
}

describe('readFromDiskPlugin', () => {
  supportTest.for([
    ['site/lib/discovery.ts',     ['crate/src/rules/a/mod.rs', 'crate/src/rules/b/lint.rs', 'site/pages/guide.md']],
    ['site/lib/discovery.ts?raw', []],
    ['site/lib/other.ts',         []]
  ] as const)('watches what %s reads from disk', ([module, expected], { tmpDir }) => {
    expect(watched(tree(tmpDir), module)).toStrictEqual(expected.map(file => path.join(tmpDir, file)))
  })

  supportTest.for([
    ['site/lib/discovery.ts', 'crate/src/rules/a/mod.rs\ncrate/src/rules/b/lint.rs\nsite/pages/guide.md'],
    ['site/lib/other.ts',     undefined]
  ] as const)('keys the module cache of %s on the files it reads', ([module, expected], { tmpDir }) => {
    expect(cacheKey(tree(tmpDir), module)).toBe(expected)
  })

  supportTest.for([
    ['a reader', { 'site/lib/gone.ts': ['crate/src/rules/**/*.rs'] }, 'site/lib/gone.ts'],
    ['a glob',   { 'site/lib/discovery.ts': ['crate/src/rules/**/*.rs', 'crate/src/gone/**'] }, 'crate/src/gone/**']
  ] as const)('throws on %s matching no file', ([, readers, stale], { tmpDir }) => {
    expect(() => readFromDiskPlugin(readers, tree(tmpDir))).toThrow(`nothing on disk matches ${stale}`)
  })
})
