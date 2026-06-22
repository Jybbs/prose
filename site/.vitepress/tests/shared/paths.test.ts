import fs   from 'node:fs'
import path from 'node:path'

import { crateDir, primitivesDir, repoRoot, rulesDir, siteDir } from '../../lib/shared/paths'

const meta = import.meta.url

describe('repoRoot', () => {
  it('walks up to the directory holding .mise/config.toml', () => {
    expect(fs.existsSync(path.join(repoRoot(meta), '.mise', 'config.toml'))).toBe(true)
  })

  it('throws when no .mise/config.toml ancestor exists', () => {
    expect(() => repoRoot('file:///')).toThrow(/repo root not found/)
  })
})

describe('directory helpers', () => {
  it('resolve the crate, site, rules, and primitives directories under the repo', () => {
    const root = repoRoot(meta)
    expect(crateDir(meta)).toBe(path.join(root, 'crate'))
    expect(siteDir(meta)).toBe(path.join(root, 'site'))
    expect(rulesDir(meta)).toBe(path.join(root, 'site', 'rules'))
    expect(primitivesDir(meta)).toBe(path.join(root, 'site', 'primitives'))
  })
})
