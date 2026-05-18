import { execFileSync } from 'node:child_process'

import { defineLoader } from 'vitepress'

import { walkFixtures }     from '../lib/fixtures/walker'
import { repoRoot }         from '../lib/shared/paths'
import { readCargoVersion } from '../lib/shared/version'
import { withFallback }     from '../lib/shared/with-fallback'

const root = repoRoot(import.meta.url)

export interface BuildData {
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
      () => execFileSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
      'unknown'
    )
    return {
      fixtureCount: [...walkFixtures(root)].length,
      gitSha,
      version: readCargoVersion(root)
    }
  }
})
