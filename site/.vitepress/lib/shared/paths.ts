import { execFileSync }  from 'node:child_process'
import fs                from 'node:fs'
import path              from 'node:path'
import { fileURLToPath } from 'node:url'

const TARGET_DIRS = new Map<string, string>()

export function architectureDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'architecture')
}

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

export function proseBinaryPath(root: string): string {
  return path.join(targetDir(root), 'debug', 'prose')
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

// Returns the directory `cargo metadata` reports for the workspace at `root`,
// reading it once per root, which `CARGO_TARGET_DIR` or a cargo config moves
// away from `<root>/target`.
function targetDir(root: string): string {
  const known = TARGET_DIRS.get(root)
  if (known) return known
  const metadata = execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: root, encoding: 'utf8' })
  const { target_directory: directory } = JSON.parse(metadata) as { target_directory: string }
  TARGET_DIRS.set(root, directory)
  return directory
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

export function viteCacheDir(metaUrl: string): string {
  return cacheDirFrom(repoRoot(metaUrl), 'vite')
}

export function vitepressCacheDir(metaUrl: string): string {
  return cacheDirFrom(repoRoot(metaUrl), 'vitepress')
}
