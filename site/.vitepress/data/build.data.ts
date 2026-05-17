import { execSync } from 'node:child_process'

import { defineLoader } from 'vitepress'

import { walkFixtures }     from '../lib/fixtures'
import { repoRoot }         from '../lib/paths'
import { readCargoVersion } from '../lib/version'
import { withFallback }     from '../lib/with-fallback'

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
      () => execSync('git rev-parse --short HEAD', { cwd: root }).toString().trim(),
      'unknown'
    )
    return {
      fixtureCount: [...walkFixtures(root)].length,
      gitSha,
      version: readCargoVersion(root)
    }
  }
})
