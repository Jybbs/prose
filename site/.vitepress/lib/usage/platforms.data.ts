import path from 'node:path'

import { defineLoader } from 'vitepress'

import { repoRoot }      from '../shared/paths'
import { requireString } from '../shared/require-string'
import { parseToml }     from '../shared/toml'

export interface WheelPlatform {
  label     : string
  manylinux : boolean
  target    : string
}

declare const data: readonly WheelPlatform[]
export { data }

const manifest = path.join(repoRoot(import.meta.url), '.github', 'scripts', 'platforms.toml')

export default defineLoader({
  watch: [manifest],
  load(): readonly WheelPlatform[] {
    const parsed = parseToml(manifest) as { platforms?: readonly Record<string, unknown>[] }
    // An entry with no target builds no wheel, the source distribution's row.
    return (parsed.platforms ?? [])
      .filter(platform => typeof platform.target === 'string')
      .map((platform, i) => ({
        label     : requireString(platform.label, `platforms.toml wheel entry ${i} has no label`),
        manylinux : typeof platform.manylinux === 'string',
        target    : platform.target as string
      }))
  }
})
