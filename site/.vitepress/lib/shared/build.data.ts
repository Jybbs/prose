import { execFileSync } from 'node:child_process'

import { defineLoader } from 'vitepress'

import { walkFixtures }     from '../fixtures/walker'
import { crateDir }         from './paths'
import { readCargoVersion } from './version'
import { withFallback }     from './with-fallback'

const crate = crateDir(import.meta.url)

interface BuildData {
  fixtureCount: number
  gitSha      : string
  version     : string
}

declare const data: BuildData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<BuildData> {
    const gitSha = await withFallback(
      'build:git-sha',
      () => execFileSync(
        'git', ['rev-parse', '--short', 'HEAD'], { cwd: crate, encoding: 'utf8' }
      ).trim(),
      'unknown'
    )
    return {
      fixtureCount: Iterator.from(walkFixtures(crate)).reduce(n => n + 1, 0),
      gitSha,
      version: readCargoVersion(crate)
    }
  }
})
