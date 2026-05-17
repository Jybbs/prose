import { execSync } from 'node:child_process'

import { defineLoader } from 'vitepress'

import { walkFixtures }    from '../lib/fixtures'
import { repoRoot }        from '../lib/paths'
import { readCargoVersion } from '../lib/version'

const root = repoRoot(import.meta.url)

export interface BuildData {
  fixtureCount: number
  gitSha      : string
  version     : string
}

declare const data: BuildData
export { data }

function gitSha(): string {
  try {
    return execSync('git rev-parse --short HEAD', { cwd: root }).toString().trim()
  } catch {
    return 'unknown'
  }
}

export default defineLoader({
  watch: [],
  load(): BuildData {
    return {
      fixtureCount: [...walkFixtures(root)].length,
      gitSha      : gitSha(),
      version     : readCargoVersion(root)
    }
  }
})
