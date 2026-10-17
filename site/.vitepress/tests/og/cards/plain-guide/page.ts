import type { OgPage } from '../../../../lib/og/pages'
import { SECTIONS }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : SECTIONS.guide,
  breadcrumb : ['Guide'],
  kind       : 'guide',
  outputPath : 'og/guide/install.png',
  title      : 'Install'
}
