import path from 'node:path'

import { requireString } from './require-string'
import { parseToml }     from './toml'

export function readCargoVersion(crateDir: string): string {
  const cargoPath = path.join(crateDir, 'Cargo.toml')
  const parsed    = parseToml(cargoPath) as { package?: { version?: unknown } }
  return requireString(parsed.package?.version, `Could not find package.version in ${cargoPath}`)
}

export function readRequiresPython(crateDir: string): string {
  const projectPath = path.join(crateDir, 'pyproject.toml')
  const parsed      = parseToml(projectPath) as { project?: { 'requires-python'?: unknown } }
  const bound       = requireString(
    parsed.project?.['requires-python'],
    `Could not find project.requires-python in ${projectPath}`
  )
  return bound.startsWith('>=') ? bound.slice(2) : bound
}

export function readRuffRelease(crateDir: string): string {
  const cargoPath = path.join(crateDir, 'Cargo.toml')
  const parsed    = parseToml(cargoPath) as {
    package?: { metadata?: { ruff?: { release?: unknown } } }
  }
  return requireString(
    parsed.package?.metadata?.ruff?.release,
    `Could not find package.metadata.ruff.release in ${cargoPath}`
  )
}
