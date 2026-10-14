import fs   from 'node:fs'
import path from 'node:path'

import * as platforms  from '../../lib/usage/platforms'
import { supportTest } from '../support'

// Writes `toml` as the manifest a repository root at `root` carries.
function manifest(root: string, toml: string): string {
  fs.mkdirSync(path.dirname(platforms.platformsManifest(root)), { recursive: true })
  fs.writeFileSync(platforms.platformsManifest(root), toml)
  return root
}

describe('readPlatforms', () => {
  supportTest('splits each wheel label and skips the entry that builds no wheel', ({ tmpDir }) => {
    const root = manifest(tmpDir, [
      '[[platforms]]', 'label = "🐧 Linux x86_64"', 'manylinux = "2_17"', 'target = "x86_64-unknown-linux-gnu"',
      '[[platforms]]', 'label = "📦 Source distribution"',
      '[[platforms]]', 'label = "🍎 macOS aarch64"', 'target = "aarch64-apple-darwin"'
    ].join('\n'))
    expect(platforms.readPlatforms(root)).toStrictEqual([
      { emoji: '🐧', label: 'Linux x86_64',  manylinux: true,  name: 'Linux', target: 'x86_64-unknown-linux-gnu' },
      { emoji: '🍎', label: 'macOS aarch64', manylinux: false, name: 'macOS', target: 'aarch64-apple-darwin' }
    ])
  })

  supportTest('reads no wheel from a manifest that lists no platforms', ({ tmpDir }) => {
    expect(platforms.readPlatforms(manifest(tmpDir, 'title = "none"'))).toStrictEqual([])
  })

  supportTest('fails on a wheel entry with no label', ({ tmpDir }) => {
    const root = manifest(tmpDir, '[[platforms]]\ntarget = "x86_64-pc-windows-msvc"')
    expect(() => platforms.readPlatforms(root)).toThrow('platforms.toml wheel entry 0 has no label')
  })
})
