import type { Scheme } from '../shared/palette'

// Names each scheme as a reader sees it.
export const SCHEME_TEXT: Record<Scheme, string> = { dark: 'Dark', light: 'Light' }

// Lists each color role the roles passage draws with, beside the custom
// property the site reads it through, in the order the legend names them.
export const ROLES = [
  { id: 'page',        label: 'Page ground',        token: '--vp-c-bg' },
  { id: 'soft',        label: 'Soft ground',        token: '--vp-c-bg-soft' },
  { id: 'text-1',      label: 'Body text',          token: '--vp-c-text-1' },
  { id: 'text-2',      label: 'Secondary text',     token: '--vp-c-text-2' },
  { id: 'text-3',      label: 'Muted text',         token: '--vp-c-text-3' },
  { id: 'link',        label: 'Link',               token: '--vp-c-brand-1' },
  { id: 'aside',       label: 'Aside',              token: '--color-aside' },
  { id: 'divider',     label: 'Divider',            token: '--vp-c-divider' },
  { id: 'code-ground', label: 'Inline code ground', token: '--vp-code-bg' },
  { id: 'code-ink',    label: 'Inline code ink',    token: '--vp-code-color' },
  { id: 'code-block',  label: 'Code block ground',  token: '--vp-code-block-bg' }
] as const

export type RoleId = (typeof ROLES)[number]['id']
