import path from 'node:path'

import { requireString } from '../shared/require-string'
import { parseToml }     from '../shared/toml'

export interface WheelPlatform {
  emoji     : string
  label     : string
  manylinux : boolean
  name      : string
  target    : string
}

export function platformsManifest(root: string): string {
  return path.join(root, '.github', 'scripts', 'platforms.toml')
}

// Reads each wheel target `platforms.toml` lists, skipping the entry with no
// target, which is the source distribution's row, and splits its label into
// the leading emoji, the platform's name, and the text after the emoji.
export function readPlatforms(root: string): readonly WheelPlatform[] {
  const parsed = parseToml(platformsManifest(root)) as { platforms?: readonly Record<string, unknown>[] }
  return (parsed.platforms ?? [])
    .filter(platform => typeof platform.target === 'string')
    .map((platform, i) => {
      const [emoji, ...words] = requireString(
        platform.label, `platforms.toml wheel entry ${i} has no label`
      ).split(' ')
      return {
        emoji     : emoji,
        label     : words.join(' '),
        manylinux : typeof platform.manylinux === 'string',
        name      : words[0],
        target    : platform.target as string
      }
    })
}
