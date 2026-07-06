import fs   from 'node:fs'
import path from 'node:path'

import { icons as logos }         from '@iconify-json/logos'
import { icons as simpleIcons }   from '@iconify-json/simple-icons'
import type { IconifyJSON }       from '@iconify/types'
import { getIconData, iconToSVG } from '@iconify/utils'
import { defineLoader }           from 'vitepress'

import { lookup }                    from '../lib/shared/lookup'
import { repoRoot }                  from '../lib/shared/paths'
import { svgViewBox }                from '../lib/shared/svg-view-box'
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

const ICON_SETS: Record<string, IconifyJSON> = {
  'logos'        : logos,
  'simple-icons' : simpleIcons
}

const repoDir = repoRoot(import.meta.url)

function loadLocalSvg(relative: string): ToolIcon {
  const file    = path.join(repoDir, 'site', '.vitepress', 'assets', relative)
  const raw     = fs.readFileSync(file, 'utf8')
  const viewBox = svgViewBox(raw, relative)
  const body    = raw
    .replaceAll(/<\?xml[^?]*\?>/g, '')
    .replaceAll(/<!--[\s\S]*?-->/g, '')
    .replace(/<svg[^>]*>/, '')
    .replace(/<\/svg>\s*$/, '')
    .trim()
  return { body: `<g fill="currentColor">${body}</g>`, viewBox }
}

const CUSTOM_ICONS: Record<string, ToolIcon> = {
  mise: loadLocalSvg('mise-logo.svg')
}

function loadIcon(spec: string): ToolIcon {
  const [pack, name] = spec.split(':')
  if (pack === 'custom') return lookup(CUSTOM_ICONS, name, 'Custom icon')
  const icon = getIconData(lookup(ICON_SETS, pack, 'Icon set'), name)
  if (icon === null) {
    throw new Error(`tools.data: icon "${spec}" not found in @iconify-json/${pack}`)
  }
  const svg = iconToSVG(icon)
  return { body: svg.body, viewBox: svg.attributes.viewBox }
}

declare const data: ToolsData
export { data }

export default defineLoader({
  watch: [],
  load(): ToolsData {
    return {
      entries: Object.fromEntries(
        Object.entries(TOOL_SEEDS).map(([slug, seed]) =>
          [slug, { ...seed, icon: loadIcon(seed.icon) }]
        )
      ) as Record<ToolSlug, ToolEntry>
    }
  }
})
