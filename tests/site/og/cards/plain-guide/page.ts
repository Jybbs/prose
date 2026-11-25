import type { OgPage } from '../../../../../site/.vitepress/lib/og/pages'
import { SECTIONS }    from '../../../../../site/.vitepress/lib/shared/palette'

export const page: OgPage = {
  accent     : SECTIONS.guide,
  breadcrumb : ['Guide'],
  kind       : 'guide',
  outputPath : 'og/guide/install.png',
  title      : 'Install'
}
