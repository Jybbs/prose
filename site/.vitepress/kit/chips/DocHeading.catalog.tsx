import type { Declaration } from '../catalog'
import { DocHeading }       from './DocHeading'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'A section heading',
      render : () => <DocHeading class="ignore-header" id="doc-heading-example" title="A Section Heading" />
    }
  ],
  use      : 'Renders a section heading beside its permalink, for a section a component writes in place of Markdown.'
}
