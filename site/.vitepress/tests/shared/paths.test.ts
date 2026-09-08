import fs                from 'node:fs'
import path              from 'node:path'
import { pathToFileURL } from 'node:url'

import * as paths          from '../../lib/shared/paths'
import { supportTest }     from '../support'

const meta = import.meta.url

describe('repoRoot', () => {
  it('walks up to the directory holding .git', () => {
    expect(fs.existsSync(path.join(paths.repoRoot(meta), '.git'))).toBe(true)
  })

  supportTest('stops at the nearest ancestor carrying .git', ({ tmpDir }) => {
    fs.writeFileSync(path.join(tmpDir, '.git'), '')
    const nested = path.join(tmpDir, 'a', 'b')
    fs.mkdirSync(nested, { recursive: true })
    expect(paths.repoRoot(pathToFileURL(path.join(nested, 'probe.ts')).href)).toBe(tmpDir)
  })

  it('throws when no .git ancestor exists', () => {
    expect(() => paths.repoRoot('file:///')).toThrow(/repo root not found/)
  })
})

describe('directory helpers', () => {
  it('resolve the crate, site, rules, primitives, and cache directories under the repo', () => {
    const root = paths.repoRoot(meta)
    expect(paths.crateDir(meta)).toBe(path.join(root, 'crate'))
    expect(paths.siteDir(meta)).toBe(path.join(root, 'site'))
    expect(paths.rulesDir(meta)).toBe(path.join(root, 'site', 'rules'))
    expect(paths.primitivesDir(meta)).toBe(path.join(root, 'site', 'primitives'))
    expect(paths.cacheDirFrom(root, 'og')).toBe(path.join(root, '.cache', 'og'))
    expect(paths.fetchCacheDir(meta)).toBe(path.join(root, '.cache', 'fetch'))
    expect(paths.vitepressCacheDir(meta)).toBe(path.join(root, '.cache', 'vitepress'))
  })
})
