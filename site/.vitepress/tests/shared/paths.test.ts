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

describe('proseBinaryPath', () => {
  // Writes the smallest crate `cargo metadata` reads at `root`.
  function crateAt(root: string): void {
    fs.mkdirSync(path.join(root, 'src'))
    fs.writeFileSync(path.join(root, 'Cargo.toml'), '[package]\nname = "probe"\nversion = "0.0.0"\n')
    fs.writeFileSync(path.join(root, 'src', 'lib.rs'), '')
  }

  supportTest('reads the binary under the target directory cargo reports', ({ tmpDir }) => {
    crateAt(tmpDir)
    vi.stubEnv('CARGO_TARGET_DIR', path.join(tmpDir, 'elsewhere'))
    expect(paths.proseBinaryPath(tmpDir)).toBe(path.join(tmpDir, 'elsewhere', 'debug', 'prose'))
  })

  supportTest('asks cargo once for each root', ({ tmpDir }) => {
    crateAt(tmpDir)
    vi.stubEnv('CARGO_TARGET_DIR', path.join(tmpDir, 'first'))
    const first = paths.proseBinaryPath(tmpDir)
    vi.stubEnv('CARGO_TARGET_DIR', path.join(tmpDir, 'second'))
    expect(paths.proseBinaryPath(tmpDir)).toBe(first)
  })
})

describe('directory helpers', () => {
  it('resolve the crate, site, rules, architecture, and cache directories under the repo', () => {
    const root = paths.repoRoot(meta)
    expect(paths.crateDir(meta)).toBe(path.join(root, 'crate'))
    expect(paths.siteDir(meta)).toBe(path.join(root, 'site'))
    expect(paths.rulesDir(meta)).toBe(path.join(root, 'site', 'rules'))
    expect(paths.architectureDir(meta)).toBe(path.join(root, 'site', 'architecture'))
    expect(paths.cacheDirFrom(root, 'og')).toBe(path.join(root, '.cache', 'og'))
    expect(paths.fetchCacheDir(meta)).toBe(path.join(root, '.cache', 'fetch'))
    expect(paths.viteCacheDir(meta)).toBe(path.join(root, '.cache', 'vite'))
    expect(paths.vitepressCacheDir(meta)).toBe(path.join(root, '.cache', 'vitepress'))
  })
})
