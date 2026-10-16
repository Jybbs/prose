import { defineLoader } from 'vitepress'

import { repoRoot }   from '../shared/paths'
import * as platforms from './platforms'

declare const data: readonly platforms.WheelPlatform[]
export { data }

const root = repoRoot(import.meta.url)

export default defineLoader({
  watch: [platforms.platformsManifest(root)],
  load(): readonly platforms.WheelPlatform[] {
    return platforms.readPlatforms(root)
  }
})
