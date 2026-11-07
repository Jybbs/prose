import type { Scheme } from '../shared/palette'

// Names each scheme as a reader sees it.
export const SCHEME_TEXT: Record<Scheme, string> = { dark: 'Dark', light: 'Light' }

// Lists the schemes in the order a specimen reads them out, light first.
export const SCHEME_ORDER = ['light', 'dark'] as const satisfies readonly Scheme[]

// Lists the least contrast each level takes, from the highest level down.
export const LEVELS = [['text', 4.5], ['mark', 3]] as const

export type Level = 'fails' | (typeof LEVELS)[number][0]

export interface Contrast {
  level : Level
  ratio : number
}

// Names each group of the roles legend, in the order the legend reads.
export const ROLE_GROUPS = { text: 'Text', marks: 'Callout marks', grounds: 'Grounds' } as const

type RoleGroup = keyof typeof ROLE_GROUPS

// Lists each color role the roles passage draws with, beside the custom
// property the site reads it through and the level its contrast is judged at,
// a ground judged by the `ink` it carries and every other role against the
// page ground. The callout marks join the `marks` group from the stylesheet
// that declares them.
export const ROLES = [
  { group: 'text',    id: 'link',        judged: 'mark', label: 'Link',           token: '--vp-c-brand-1' },
  { group: 'text',    id: 'link-hover',  judged: 'text', label: 'Link hover',     token: '--vp-c-brand-2' },
  { group: 'text',    id: 'aside',       judged: 'text', label: 'Aside',          token: '--color-aside' },
  { group: 'text',    id: 'text-1',      judged: 'text', label: 'Body text',      token: '--vp-c-text-1' },
  { group: 'text',    id: 'text-2',      judged: 'text', label: 'Secondary text', token: '--vp-c-text-2' },
  { group: 'text',    id: 'text-3',      judged: 'text', label: 'Muted text',     token: '--vp-c-text-3' },
  { group: 'grounds', id: 'page',        judged: 'text', label: 'Page ground',    token: '--vp-c-bg',          ink: '--vp-c-text-1' },
  { group: 'grounds', id: 'soft',        judged: 'text', label: 'Soft ground',    token: '--vp-c-bg-soft',     ink: '--vp-c-text-1' },
  { group: 'grounds', id: 'code-block',  judged: 'text', label: 'Code block',     token: '--vp-code-block-bg', ink: '--vp-c-text-1' },
  { group: 'grounds', id: 'code-ground', judged: 'text', label: 'Inline code',    token: '--vp-code-bg',       ink: '--vp-code-color' }
] as const satisfies readonly RoleDeclaration[]

// Lists the properties the passage reads beyond the roles' own, each set to
// its value in the scheme the passage is drawn in.
export const PASSAGE_TOKENS = ['--color-link', '--vp-c-divider'] as const

export interface RoleDeclaration {
  group  : RoleGroup
  id     : string
  ink   ?: string
  judged : Exclude<Level, 'fails'>
  label  : string
  token  : string
}

// Holds one role as the legend sets it, with its value and its contrast in
// each scheme.
export interface Role {
  contrast : Record<Scheme, Contrast>
  group    : RoleGroup
  id       : string
  judged   : Exclude<Level, 'fails'>
  label    : string
  token    : string
  value    : Record<Scheme, string>
}
