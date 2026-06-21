import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'smol-toml'

export function readCargoVersion(crateDir: string): string {
  const cargoPath = path.join(crateDir, 'Cargo.toml')
  const body      = fs.readFileSync(cargoPath, 'utf8')
  const parsed    = parse(body) as { package?: { version?: unknown } }
  const version   = parsed.package?.version
  if (typeof version !== 'string') {
    throw new Error(`Could not find package.version in ${cargoPath}`)
  }
  return version
}
