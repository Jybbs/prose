import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'smol-toml'

import { requireString } from './require-string'

export function readCargoVersion(crateDir: string): string {
  const cargoPath = path.join(crateDir, 'Cargo.toml')
  const parsed    = parse(fs.readFileSync(cargoPath, 'utf8')) as { package?: { version?: unknown } }
  return requireString(parsed.package?.version, `Could not find package.version in ${cargoPath}`)
}
