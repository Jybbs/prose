import { repoRoot }         from '../lib/shared/paths'
import { readCargoVersion } from '../lib/shared/version'
import { fixtureDir }       from './support'

describe('readCargoVersion', () => {
  it('reads the crate version from Cargo.toml', () => {
    expect(readCargoVersion(repoRoot(import.meta.url))).toMatch(/^\d+\.\d+\.\d+/)
  })

  it('throws when the manifest carries no package version', () => {
    expect(() => readCargoVersion(fixtureDir('cargo-no-version'))).toThrow(/package\.version/)
  })
})
