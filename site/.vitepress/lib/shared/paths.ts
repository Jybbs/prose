import fs   from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

let cachedRoot: string | null = null

export function repoRoot(metaUrl: string): string {
  if (cachedRoot !== null) return cachedRoot
  let dir = path.dirname(fileURLToPath(metaUrl))
  while (!fs.existsSync(path.join(dir, 'package.json'))) {
    const parent = path.dirname(dir)
    if (parent === dir) throw new Error(`repo root not found from ${metaUrl}`)
    dir = parent
  }
  cachedRoot = dir
  return dir
}

export function rulesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'rules')
}

export function siteDir(metaUrl: string): string {
  return path.join(repoRoot(metaUrl), 'site')
}
