import type { OgPage } from '../../../../lib/og/pages'
import { SECTIONS }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : SECTIONS.architecture,
  breadcrumb : ['Architecture'],
  kind       : 'architecture',
  outputPath : 'og/architecture/text-range.png',
  primitive  : { stability: 'public' },
  title      : 'The TextRange Type'
}
