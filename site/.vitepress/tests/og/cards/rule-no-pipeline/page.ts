import type { OgPage } from '../../../../lib/og/pages'
import { FAMILIES }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : FAMILIES.alignment,
  breadcrumb : ['Rules', 'Alignment'],
  caption    : 'Aligns `=` across runs',
  category   : 'auto-fix',
  family     : 'alignment',
  kind       : 'rules',
  outputPath : 'og/rules/alignment/align-equals.png',
  title      : 'Align Equals'
}
