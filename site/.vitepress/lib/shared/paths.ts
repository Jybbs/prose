import fs                from 'node:fs'
import path              from 'node:path'
import { fileURLToPath } from 'node:url'

export function repoRoot(metaUrl: string): string {
  let dir = path.dirname(fileURLToPath(metaUrl))
  while (!fs.existsSync(path.join(dir, '.mise', 'config.toml'))) {
    const parent = path.dirname(dir)
    if (parent === dir) throw new Error(`repo root not found from ${metaUrl}`)
    dir = parent
  }
  return dir
}

export function crateDir(metaUrl: string): string {
  return crateDirFrom(repoRoot(metaUrl))
}

export function crateDirFrom(repoRoot: string): string {
  return path.join(repoRoot, 'crate')
}

export function primitivesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'primitives')
}

export function proseBinaryCandidates(repoRoot: string): string[] {
  return ['target/release/prose', 'target/debug/prose'].map(p => path.join(repoRoot, p))
}

export function resolveProseBinary(repoRoot: string): string {
  const found = proseBinaryCandidates(repoRoot).find(fs.existsSync)
  if (found) return found
  throw new Error('prose binary not found at target/{release,debug}/prose. Run `cargo build` first.')
}

export function rulesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'rules')
}

export function siteDir(metaUrl: string): string {
  return path.join(repoRoot(metaUrl), 'site')
}
