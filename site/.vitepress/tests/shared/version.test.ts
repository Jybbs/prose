import { crateDir }         from '../../lib/shared/paths'
import { readCargoVersion } from '../../lib/shared/version'
import { fixtureDir }       from '../support'

describe('readCargoVersion', () => {
  it('reads the crate version from Cargo.toml', () => {
    expect(readCargoVersion(crateDir(import.meta.url))).toMatch(/^\d+\.\d+\.\d+/)
  })

  it('throws when the manifest carries no package version', () => {
    const dir = fixtureDir(import.meta.dirname, 'cargo-no-version')
    expect(() => readCargoVersion(dir)).toThrow(/package\.version/)
  })
})
