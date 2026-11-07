import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { KINDS }                              from '../markdown/callout-plugin'
import { discoverRuleSlugs }                  from '../rules/discovery'
import { groupByMember }                      from '../shared/group-by-member'
import * as palette                           from '../shared/palette'
import { rulesDir, siteDir, VITEPRESS_THEME } from '../shared/paths'
import { FAMILY_ORDER }                       from '../shared/registries'
import { toTitleCase }                        from '../shared/title-case'
import { Cascade }                            from './cascade'
import { readMarks }                          from './mark-files'
import { PASSAGE_TOKENS, ROLES }              from './roles'
import * as swatchSet                         from './swatches'

import type { Scheme }                      from '../shared/palette'
import type { GlossaryFamily, RuleFamily }  from '../shared/registries'
import type { Marks }                       from './marks'
import type { Role, RoleDeclaration }       from './roles'
import type { GroupId, SourceFile, Swatch } from './swatches'

export interface Identity {
  brand      : string
  groups     : readonly { id: GroupId, title: string }[]
  inkRatio   : number
  inks       : Record<GlossaryFamily, Record<Scheme, string>>
  marks      : Marks
  page       : Record<Scheme, string>
  passage    : Record<Scheme, Record<string, string>>
  roles      : readonly Role[]
  ruleCounts : Record<RuleFamily, number>
  schemes    : readonly Scheme[]
  swatches   : readonly Swatch[]
}

declare const data: Identity
export { data }

const site      = siteDir(import.meta.url)
const VITEPRESS = path.join(site, VITEPRESS_THEME, 'styles/vars.css')
const TOKENS    = path.join(site, '.vitepress/theme/styles/tokens.css')
const CALLOUTS  = path.join(site, '.vitepress/theme/styles/markdown/callouts.css')

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
    const uses    = swatchSet.scanUses(
      sources(['{kit,theme}/**/*.css']),
      sources(['{kit,lib,theme}/**/*.{ts,tsx}', 'config.ts']).filter(({ file }) => !file.endsWith('palette.ts') && !file.includes('/identity/'))
    )
    const marks   = swatchSet.calloutMarks(fs.readFileSync(CALLOUTS, 'utf8'))
    const page    = cascade.value('--vp-c-bg')
    const declared: readonly RoleDeclaration[] = [...ROLES, ...[...marks].map(([kind, token]) => ({
      group  : 'marks',
      id     : `callout-${kind}`,
      judged : 'mark',
      label  : KINDS[kind as keyof typeof KINDS].label,
      token
    }) as const)]
    const roles   = declared.map(({ ink, ...role }): Role => {
      const value          = cascade.value(role.token)
      const [fore, ground] = ink ? [cascade.value(ink), value] : [value, page]
      return { ...role, contrast: palette.perScheme(scheme => swatchSet.contrastOn(fore[scheme], ground[scheme])), value }
    })
    const tokens  = new Set([...declared.flatMap(role => role.ink ? [role.token, role.ink] : [role.token]), ...PASSAGE_TOKENS])
    const all     = swatchSet.swatches(uses, page, swatchSet.calloutStates(marks))
    return {
      brand      : toTitleCase(palette.BRAND, '-'),
      groups     : swatchSet.GROUPS.filter(id => all.some(swatch => swatch.group === id)).map(id => ({ id, title: toTitleCase(id, '-') })),
      inkRatio   : palette.ACCENT_RATIOS.nib,
      inks       : Object.fromEntries(Object.keys(palette.FAMILIES).map(family => [family, palette.perScheme(scheme =>
        palette.SCHEME_SHADES[scheme][palette.shadeName(family, 'page', palette.ACCENT_RATIOS.nib)]!)])) as Identity['inks'],
      marks      : readMarks(file => fs.readFileSync(path.join(site, 'public', file), 'utf8')),
      page,
      passage    : palette.perScheme(scheme => Object.fromEntries([...tokens].map(token => [token, cascade.value(token)[scheme]]))),
      roles,
      ruleCounts : ruleCounts(),
      schemes    : palette.SCHEMES,
      swatches   : all
    }
  }
})
