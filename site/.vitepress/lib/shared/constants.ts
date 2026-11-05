export const BODY_LINK_CLASS = 'body-link'
export const CATALOG_SUFFIX  = '.catalog.tsx'
export const MS_PER_SEC      = 1000
export const PYPI_PACKAGE    = 'prose-formatter'
export const REPO_SLUG       = 'Jybbs/prose'
export const REPO_URL        = `https://github.com/${REPO_SLUG}`
export const SHIKI_THEMES    = { dark: 'github-dark', light: 'github-light' } as const
export const SITE_ALT        = 'Prose, a Python typesetter for the reader.'
export const SITE_HOSTNAME   = 'https://prose.fyi'
export const SITE_NAME       = 'Prose'
export const SITE_TAGLINE    = 'A Python typesetter for the reader.'

// Maps each token color of the two themes that reads under 4.5:1 on the site's
// code grounds to the nearest color that reaches it, and takes the place of the
// map VitePress applies to the markdown highlighter alone.
export const SHIKI_COLOR_REPLACEMENTS = {
  'github-dark'  : { '#2188ff': '#268bf9', '#586069': '#5b93a3', '#6a737d': '#84909c', '#ea4a5a': '#ef5564' },
  'github-light' : {
    '#0366d6' : '#0663d0',
    '#1b7c83' : '#06747a',
    '#22863a' : '#11782a',
    '#28a745' : '#0e790b',
    '#3192aa' : '#05728b',
    '#6a737d' : '#62687b',
    '#959da5' : '#6c676f',
    '#b08800' : '#846312',
    '#cb2431' : '#c82430',
    '#d73a49' : '#c62739',
    '#e36209' : '#c13617'
  }
}
