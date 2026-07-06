import type { OgPage } from '../../../../lib/og/pages'
import { SECTIONS }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : SECTIONS.usage,
  breadcrumb : ['Usage'],
  kind       : 'usage',
  outputPath : 'og/usage/install.png',
  title      : 'Install'
}
