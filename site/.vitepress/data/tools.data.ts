import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { repoRoot }                  from '../lib/shared/paths'
import { TOOL_SEEDS, type ToolSlug } from '../lib/shared/tools'

interface ToolIcon {
  body    : string
  viewBox : string
}

interface ToolEntry {
  href : string
  icon : ToolIcon
  name : string
  role : string
}

interface ToolsData {
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

const repoDir = repoRoot(import.meta.url)

const loadPack = (pack: string): IconPack => {
  const file = path.join(repoDir, 'site', 'node_modules', `@iconify-json/${pack}/icons.json`)
  return JSON.parse(fs.readFileSync(file, 'utf8')) as IconPack
}

function loadLocalSvg(relative: string, viewBox: string): ToolIcon {
  const file = path.join(repoDir, 'site', '.vitepress', 'assets', relative)
  const raw  = fs.readFileSync(file, 'utf8')
  const body = raw
    .replace(/<\?xml[^?]*\?>/g, '')
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<svg[^>]*>/, '')
    .replace(/<\/svg>\s*$/, '')
    .trim()
  return { body: `<g fill="currentColor">${body}</g>`, viewBox }
}

const CUSTOM_ICONS: Record<string, ToolIcon> = {
  mise: loadLocalSvg('mise-logo.svg', '50 35 205 230')
}

function loadIcon(spec: string): ToolIcon {
  const [pack, name] = spec.split(':')
  if (pack === 'custom') {
    const custom = CUSTOM_ICONS[name]
    if (custom === undefined) {
      throw new Error(`tools.data: custom icon "${name}" not registered`)
    }
    return custom
  }
  const data  = loadPack(pack)
  const entry = data.icons[name]
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
        Object.entries(TOOL_SEEDS).map(([slug, seed]) =>
          [slug, { ...seed, icon: loadIcon(seed.icon) }]
        )
      ) as Record<ToolSlug, ToolEntry>
    }
  }
})
