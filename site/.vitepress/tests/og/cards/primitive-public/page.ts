import type { OgPage } from '../../../../lib/og/pages'
import { SECTIONS }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : SECTIONS.primitives,
  breadcrumb : ['Primitives'],
  kind       : 'primitives',
  outputPath : 'og/primitives/text-range.png',
  primitive  : { stability: 'public' },
  title      : 'Text Range'
}
