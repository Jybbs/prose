import fs   from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'

import { defineLoader } from 'vitepress'

import { repoRoot }          from '../lib/paths'
import { readCargoVersion }  from '../lib/version'

const root = repoRoot(import.meta.url)

export interface BuildData {
  builtAt     : string
  fixtureCount: number
  gitSha      : string
  version     : string
}

declare const data: BuildData
export { data }

function fixtureCount(): number {
  const fixturesRoot = path.join(root, 'tests/fixtures')
  let count          = 0
  for (const rule of fs.readdirSync(fixturesRoot)) {
    const dir = path.join(fixturesRoot, rule)
    if (!fs.statSync(dir).isDirectory()) continue
    for (const file of fs.readdirSync(dir)) {
      if (file.endsWith('.input.py')) count++
    }
  }
  return count
}

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
      builtAt     : new Date().toISOString().slice(0, 10),
      fixtureCount: fixtureCount(),
      gitSha      : gitSha(),
      version     : readCargoVersion(root)
    }
  }
})
