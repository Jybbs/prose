import type { OgPage } from '../../../../lib/og/pages'
import { FAMILIES }    from '../../../../lib/shared/palette'

export const page: OgPage = {
  accent     : FAMILIES.alignment,
  breadcrumb : ['Rules', 'Alignment'],
  category   : 'auto-fix',
  family     : 'alignment',
  kind       : 'rules',
  outputPath : 'og/rules/alignment/long.png',
  pipeline   : { position: 1, total: 20 },
  title      : 'xxxxxxxxxxxxxxxxxxxx'
}
