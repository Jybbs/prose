import { existsSync }    from 'node:fs'
import path              from 'node:path'
import { fileURLToPath } from 'node:url'

// Build-time filesystem anchors. The functions derive absolute paths from a
// loader or integration `config.root`, the site directory Astro resolves the
// build against, so the `node:fs` and `node:child_process` reads reach the
// crate and the compiled binary that sit beside the site in the workspace.

export const DOCS_CONTENT_DIR = 'src/content/docs/'

export function repoRoot(siteRoot: URL): string {
  return fileURLToPath(new URL('../', siteRoot))
}

function crateDir(siteRoot: URL): string {
  return fileURLToPath(new URL('../crate/', siteRoot))
}

export function cargoTomlPath(siteRoot: URL): string {
  return path.join(crateDir(siteRoot), 'Cargo.toml')
}

export function fixturesDir(siteRoot: URL): string {
  return path.join(crateDir(siteRoot), 'tests', 'fixtures')
}

export function ruleSourcePath(siteRoot: URL): string {
  return path.join(crateDir(siteRoot), 'src', 'rule.rs')
}

export function proseBinaryCandidates(siteRoot: URL): string[] {
  const root = repoRoot(siteRoot)
  return ['target/release/prose', 'target/debug/prose'].map(rel => path.join(root, rel))
}

export function resolveProseBinary(siteRoot: URL): string {
  const found = proseBinaryCandidates(siteRoot).find(existsSync)
  if (found === undefined) {
    throw new Error('prose binary not found at target/{release,debug}/prose. Run `cargo build` first.')
  }
  return found
}
