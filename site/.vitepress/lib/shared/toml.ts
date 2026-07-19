import fs from 'node:fs'

import { parse } from 'smol-toml'

export function parseToml(manifestPath: string): unknown {
  return parse(fs.readFileSync(manifestPath, 'utf8'))
}
