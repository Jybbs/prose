import { crateDir }   from '../../lib/shared/paths'
import * as version   from '../../lib/shared/version'
import { fixtureDir } from '../support'

const crate = crateDir(import.meta.url)

describe('readCargoVersion', () => {
  it('reads the crate version from Cargo.toml', () => {
    expect(version.readCargoVersion(crate)).toMatch(/^\d+\.\d+\.\d+/)
  })

  it('throws when the manifest carries no package version', () => {
    const dir = fixtureDir(import.meta.dirname, 'cargo-no-version')
    expect(() => version.readCargoVersion(dir)).toThrow(/package\.version/)
  })
})

describe('readRequiresPython', () => {
  it('reads the floor from pyproject.toml with the bound stripped', () => {
    expect(version.readRequiresPython(crate)).toMatch(/^\d+\.\d+$/)
  })

  it('throws when the project table carries no requires-python', () => {
    const dir = fixtureDir(import.meta.dirname, 'pyproject-no-floor')
    expect(() => version.readRequiresPython(dir)).toThrow(/requires-python/)
  })
})

describe('readRuffRelease', () => {
  it('reads the ruff release from Cargo.toml', () => {
    expect(version.readRuffRelease(crate)).toMatch(/^\d+\.\d+\.\d+$/)
  })

  it('throws when the manifest records no ruff release', () => {
    const dir = fixtureDir(import.meta.dirname, 'cargo-no-ruff-release')
    expect(() => version.readRuffRelease(dir)).toThrow(/package\.metadata\.ruff\.release/)
  })
})
