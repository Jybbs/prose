import type { OgPage } from '../../../../lib/og/pages'
import { FAMILIES }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : FAMILIES.alignment,
  breadcrumb : ['Rules'],
  kind       : 'rules',
  outputPath : 'og/rules/alignment.png',
  title      : 'Alignment'
}
