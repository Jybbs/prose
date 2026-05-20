import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { repoRoot }                  from '../lib/shared/paths'
import { TOOL_SEEDS, type ToolSlug } from '../lib/shared/tools'

export interface ToolIcon {
  body    : string
  viewBox : string
}

export interface ToolEntry {
  href : string
  icon : ToolIcon
  name : string
  role : string
}

export interface ToolsData {
  entries : Record<ToolSlug, ToolEntry>
}

interface IconPackEntry {
  body    : string
  height ?: number
  width  ?: number
}

interface IconPack {
  height ?: number
  icons   : Record<string, IconPackEntry>
  width  ?: number
}

const repoDir    = repoRoot(import.meta.url)
const PACK_CACHE = new Map<string, IconPack>()

function loadPack(pack: string): IconPack {
  const cached = PACK_CACHE.get(pack)
  if (cached !== undefined) return cached
  const file = path.join(repoDir, 'node_modules', `@iconify-json/${pack}/icons.json`)
  const data = JSON.parse(fs.readFileSync(file, 'utf8')) as IconPack
  PACK_CACHE.set(pack, data)
  return data
}

function loadIcon(spec: string): ToolIcon {
  const [pack, name] = spec.split(':')
  const data         = loadPack(pack)
  const entry        = data.icons[name]
  if (entry === undefined) {
    throw new Error(`tools.data: icon "${spec}" not found in @iconify-json/${pack}`)
  }
  const w = entry.width  ?? data.width  ?? 24
  const h = entry.height ?? data.height ?? 24
  return { body: entry.body, viewBox: `0 0 ${w} ${h}` }
}

declare const data: ToolsData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<ToolsData> {
    return {
      entries: Object.fromEntries(
        Object.entries(TOOL_SEEDS).map(([slug, seed]) => [slug, { ...seed, icon: loadIcon(seed.icon) }])
      ) as Record<ToolSlug, ToolEntry>
    }
  }
})
