import { siteDir }    from '../../lib/shared/paths'
import * as version   from '../../lib/shared/version'
import { CRATE }      from '../corpus'
import { fixtureDir } from '../support'

const site = siteDir(import.meta.url)

describe('readCargoVersion', () => {
  it('reads the crate version from Cargo.toml', () => {
    expect(version.readCargoVersion(CRATE)).toMatch(/^\d+\.\d+\.\d+/)
  })

  it('throws when the manifest carries no package version', () => {
    const dir = fixtureDir(import.meta.dirname, 'cargo-no-version')
    expect(() => version.readCargoVersion(dir)).toThrow(/package\.version/)
  })
})

describe('readPackageVersions', () => {
  it('reads the pinned version of each named devDependency', () => {
    const semver = expect.stringMatching(/^\d+\.\d+\.\d+/)
    expect(version.readPackageVersions(site, ['@resvg/resvg-js', 'satori']))
      .toStrictEqual({ '@resvg/resvg-js': semver, satori: semver })
  })

  it.each([
    ['throws when the manifest has no such devDependency',      'package-no-pin'],
    ['throws when the manifest has no devDependencies at all',  'package-no-dev-deps']
  ])('%s', (_name, fixture) => {
    const dir = fixtureDir(import.meta.dirname, fixture)
    expect(() => version.readPackageVersions(dir, ['satori']))
      .toThrow(/devDependencies\['satori'\]/)
  })
})

describe('readRequiresPython', () => {
  it('reads the floor from pyproject.toml with the bound stripped', () => {
    expect(version.readRequiresPython(CRATE)).toMatch(/^\d+\.\d+$/)
  })

  it('reads a floor written without a bound', () => {
    const dir = fixtureDir(import.meta.dirname, 'pyproject-bare-floor')
    expect(version.readRequiresPython(dir)).toBe('3.10')
  })

  it('throws when the project table carries no requires-python', () => {
    const dir = fixtureDir(import.meta.dirname, 'pyproject-no-floor')
    expect(() => version.readRequiresPython(dir)).toThrow(/requires-python/)
  })
})

describe('readRuffRelease', () => {
  it('reads the ruff release from Cargo.toml', () => {
    expect(version.readRuffRelease(CRATE)).toMatch(/^\d+\.\d+\.\d+$/)
  })

  it('throws when the manifest records no ruff release', () => {
    const dir = fixtureDir(import.meta.dirname, 'cargo-no-ruff-release')
    expect(() => version.readRuffRelease(dir)).toThrow(/package\.metadata\.ruff\.release/)
  })
})
