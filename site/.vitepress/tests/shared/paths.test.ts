import fs   from 'node:fs'
import path from 'node:path'

import * as paths from '../../lib/shared/paths'

const meta = import.meta.url

describe('repoRoot', () => {
  it('walks up to the directory holding .mise/config.toml', () => {
    expect(fs.existsSync(path.join(paths.repoRoot(meta), '.mise', 'config.toml'))).toBe(true)
  })

  it('throws when no .mise/config.toml ancestor exists', () => {
    expect(() => paths.repoRoot('file:///')).toThrow(/repo root not found/)
  })
})

describe('directory helpers', () => {
  it('resolve the crate, site, rules, primitives, and fixtures directories under the repo', () => {
    const root = paths.repoRoot(meta)
    expect(paths.crateDir(meta)).toBe(path.join(root, 'crate'))
    expect(paths.siteDir(meta)).toBe(path.join(root, 'site'))
    expect(paths.rulesDir(meta)).toBe(path.join(root, 'site', 'rules'))
    expect(paths.primitivesDir(meta)).toBe(path.join(root, 'site', 'primitives'))
    expect(paths.fixturesDir(meta)).toBe(path.join(root, 'crate', 'tests', 'fixtures'))
  })
})
