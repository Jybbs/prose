import type { OgPage } from '../../../../lib/og/pages'

export const page: OgPage = {
  breadcrumb : ['Rules', 'Lint'],
  caption    : 'Flags `import` used bare',
  category   : 'lint',
  family     : 'lint',
  kind       : 'rules',
  outputPath : 'og/rules/lint/bare-imports.png',
  pipeline   : { position: 18, total: 20 },
  title      : 'Bare Imports'
}
