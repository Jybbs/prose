import { execFileSync }  from 'node:child_process'
import fs                from 'node:fs'
import path              from 'node:path'
import { fileURLToPath } from 'node:url'

export function cacheDirFrom(root: string, name: string): string {
  return path.join(root, '.cache', name)
}

export function crateDir(metaUrl: string): string {
  return crateDirFrom(repoRoot(metaUrl))
}

export function crateDirFrom(root: string): string {
  return path.join(root, 'crate')
}

export function fetchCacheDir(metaUrl: string): string {
  return cacheDirFrom(repoRoot(metaUrl), 'fetch')
}

export function fixturesDirFrom(crate: string): string {
  return path.join(crate, 'tests', 'fixtures')
}

export function primitivesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'primitives')
}

export function proseBinaryPath(root: string): string {
  return path.join(root, 'target', 'debug', 'prose')
}

export function repoRoot(metaUrl: string): string {
  let dir = path.dirname(fileURLToPath(metaUrl))
  while (!fs.existsSync(path.join(dir, '.git'))) {
    const parent = path.dirname(dir)
    if (parent === dir) throw new Error(`repo root not found from ${metaUrl}`)
    dir = parent
  }
  return dir
}

export function rulesDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'rules')
}

function resolveProseBinary(root: string): string {
  const binary = proseBinaryPath(root)
  if (fs.existsSync(binary)) return binary
  throw new Error(`prose binary not found at ${binary}. Run \`mise run rust:build\` first.`)
}

export function runProse(
  root    : string,
  args    : readonly string[],
  options : { cwd?: string, input?: string, stdio?: 'pipe' } = {}
): string {
  return execFileSync(resolveProseBinary(root), args, { encoding: 'utf8', ...options })
}

export function siteDir(metaUrl: string): string {
  return path.join(repoRoot(metaUrl), 'site')
}

export function vitepressCacheDir(metaUrl: string): string {
  return cacheDirFrom(repoRoot(metaUrl), 'vitepress')
}
