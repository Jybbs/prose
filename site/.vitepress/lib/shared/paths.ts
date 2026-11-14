import { execFileSync, spawnSync } from 'node:child_process'
import fs                          from 'node:fs'
import os                          from 'node:os'
import path                        from 'node:path'
import { fileURLToPath }           from 'node:url'

// Locates the VitePress default theme under the site's directory.
export const VITEPRESS_THEME = 'node_modules/vitepress/dist/client/theme-default'

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

export function readmeDir(metaUrl: string): string {
  return path.join(siteDir(metaUrl), 'public', 'readme')
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

// Returns `CARGO_TARGET_DIR` resolved against `root`, first setting it to the
// `target_directory` that `cargo metadata` reports where it is unset, so every
// copy of this module a bundle carries reads the same directory.
function targetDir(root: string): string {
  process.env.CARGO_TARGET_DIR ??= (JSON.parse(
    execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: root, encoding: 'utf8' })
  ) as { target_directory: string }).target_directory
  return path.resolve(root, process.env.CARGO_TARGET_DIR)
}

export function runProse(
  root    : string,
  args    : readonly string[],
  options : { cwd?: string, input?: string, stdio?: 'pipe' } = {}
): string {
  return execFileSync(resolveProseBinary(root), args, { encoding: 'utf8', ...options })
}

// Runs the binary over `input` from a scratch directory holding `configToml`
// as its `prose.toml`, removing the directory once the run returns, and returns
// its stdout whatever status it exits with, since a run reporting findings exits
// non-zero.
export function readProseUnder(
  root       : string,
  args       : readonly string[],
  input      : string,
  configToml : string
): string {
  using scratch = fs.mkdtempDisposableSync(path.join(os.tmpdir(), 'prose-site-'))
  fs.writeFileSync(path.join(scratch.path, 'prose.toml'), configToml)
  const run = spawnSync(resolveProseBinary(root), args, { cwd: scratch.path, encoding: 'utf8', input })
  if (run.error) throw run.error
  return run.stdout
}

export function highlightCacheDir(metaUrl: string): string {
  return cacheDirFrom(repoRoot(metaUrl), 'highlight')
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
