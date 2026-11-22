import type { OgPage } from '../../../../../site/.vitepress/lib/og/pages'
import { FAMILIES }    from '../../../../../site/.vitepress/lib/shared/palette'

export const page: OgPage = {
  accent     : FAMILIES.ordering,
  breadcrumb : ['Rules', 'Ordering'],
  caption    : 'Sorts `import` bands into **runs**',
  family     : 'ordering',
  kind       : 'rules',
  outputPath : 'og/rules/ordering/alphabetize-siblings.png',
  pipeline   : { position: 3, total: 20 },
  title      : 'Alphabetize Siblings'
}
