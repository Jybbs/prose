import fs   from 'node:fs'
import path from 'node:path'

export function readCargoVersion(repoRoot: string): string {
  const body  = fs.readFileSync(path.join(repoRoot, 'Cargo.toml'), 'utf8')
  const match = body.match(/^\[package\][\s\S]*?^\s*version\s*=\s*"([^"]+)"/m)
  if (!match) {
    throw new Error(`Could not find package version in ${repoRoot}/Cargo.toml`)
  }
  return match[1]
}
