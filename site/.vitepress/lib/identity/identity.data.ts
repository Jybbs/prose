import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }                  from '../rules/discovery'
import { groupByMember }                      from '../shared/group-by-member'
import * as palette                           from '../shared/palette'
import { rulesDir, siteDir, VITEPRESS_THEME } from '../shared/paths'
import { FAMILY_ORDER }                       from '../shared/registries'
import { toTitleCase }                        from '../shared/title-case'
import { Cascade }                            from './cascade'
import { readMarks }                          from './mark-files'
import { ROLES }                              from './roles'
import { GROUPS, scanUses, swatches }         from './swatches'

import type { Scheme }                      from '../shared/palette'
import type { GlossaryFamily, RuleFamily }  from '../shared/registries'
import type { Marks }                       from './marks'
import type { RoleId }                      from './roles'
import type { GroupId, SourceFile, Swatch } from './swatches'

export interface Identity {
  brand      : string
  groups     : readonly { id: GroupId, title: string }[]
  inks       : Record<GlossaryFamily, Record<Scheme, string>>
  marks      : Marks
  roles      : Record<RoleId, Record<Scheme, string>>
  ruleCounts : Record<RuleFamily, number>
  schemes    : readonly Scheme[]
  swatches   : readonly Swatch[]
}

declare const data: Identity
export { data }

const site      = siteDir(import.meta.url)
const VITEPRESS = path.join(site, VITEPRESS_THEME, 'styles/vars.css')
const TOKENS    = path.join(site, '.vitepress/theme/styles/tokens.css')

// Reads every file `patterns` match under the site's `.vitepress` directory,
// named by its path there.
function sources(patterns: string[]): SourceFile[] {
  const root = path.join(site, '.vitepress')
  return fs.globSync(patterns, { cwd: root }).toSorted().map(file => ({ file, text: fs.readFileSync(path.join(root, file), 'utf8') }))
}

// Counts the rules each family holds, a family no rule carries counting zero.
function ruleCounts(): Record<RuleFamily, number> {
  const byFamily = groupByMember(discoverRuleSlugs(rulesDir(import.meta.url)), rule => rule.family, FAMILY_ORDER)
  return Object.fromEntries(FAMILY_ORDER.map(family => [family, byFamily[family].length])) as Record<RuleFamily, number>
}

export default defineLoader({
  watch: ['../../{kit,lib,theme}/**/*.{css,ts,tsx}', '../../../public/*.svg'],
  load(): Identity {
    const cascade = Cascade.read([fs.readFileSync(VITEPRESS, 'utf8'), palette.themeCss(), palette.paletteCss(), fs.readFileSync(TOKENS, 'utf8')])
    const uses    = scanUses(
      sources(['{kit,theme}/**/*.css']),
      sources(['{kit,lib,theme}/**/*.{ts,tsx}', 'config.ts']).filter(({ file }) => !file.endsWith('palette.ts') && !file.includes('/identity/'))
    )
    const roles   = Object.fromEntries(ROLES.map(role => [role.id, cascade.value(role.token)])) as Identity['roles']
    const all     = swatches(uses, roles.page)
    return {
      brand      : toTitleCase(palette.BRAND, '-'),
      groups     : GROUPS.filter(id => all.some(swatch => swatch.group === id)).map(id => ({ id, title: toTitleCase(id, '-') })),
      inks       : Object.fromEntries(Object.keys(palette.FAMILIES).map(family => [family, palette.perScheme(scheme =>
        palette.SCHEME_SHADES[scheme][palette.shadeName(family, 'page', palette.ACCENT_RATIOS.nib)]!)])) as Identity['inks'],
      marks      : readMarks(file => fs.readFileSync(path.join(site, 'public', file), 'utf8')),
      roles,
      ruleCounts : ruleCounts(),
      schemes    : palette.SCHEMES,
      swatches   : all
    }
  }
})
