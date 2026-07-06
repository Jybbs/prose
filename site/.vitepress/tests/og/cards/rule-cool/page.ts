import type { OgPage } from '../../../../lib/og/pages'
import { FAMILIES }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : FAMILIES.ordering,
  breadcrumb : ['Rules', 'Ordering'],
  caption    : 'Sorts `import` bands into **runs**',
  category   : 'auto-fix',
  family     : 'ordering',
  kind       : 'rules',
  outputPath : 'og/rules/ordering/alphabetize.png',
  pipeline   : { position: 3, total: 20 },
  title      : 'Alphabetize'
}
