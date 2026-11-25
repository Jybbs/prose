import type { OgPage } from '../../../../../site/.vitepress/lib/og/pages'
import { FAMILIES }    from '../../../../../site/.vitepress/lib/shared/palette'

export const page: OgPage = {
  accent     : FAMILIES.alignment,
  breadcrumb : ['Rules', 'Alignment'],
  caption    : 'Aligns `=` across runs',
  family     : 'alignment',
  kind       : 'rules',
  outputPath : 'og/rules/alignment/align-equals.png',
  title      : 'Align Equals'
}
