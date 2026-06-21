import fs                from 'node:fs'
import path              from 'node:path'
import { fileURLToPath } from 'node:url'

export function repoRoot(metaUrl: string): string {
  let dir = path.dirname(fileURLToPath(metaUrl))
  while (!fs.existsSync(path.join(dir, 'mise.toml'))) {
    const parent = path.dirname(dir)
    if (parent === dir) throw new Error(`repo root not found from ${metaUrl}`)
    dir = parent
  }
  return dir
}

export function primitivesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'primitives')
}

export function rulesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'rules')
}

export function siteDir(metaUrl: string): string {
  return path.join(repoRoot(metaUrl), 'site')
}
