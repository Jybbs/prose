import type { Declaration } from '../catalog'
import { ToolIcon }         from './ToolIcon'

export const declaration: Declaration = {
  examples : [
    { label: 'Icon alone', render: () => <ToolIcon class="size-14" slug="uv" /> }
  ],
  use      : 'Draws a tool icon alone by its registry slug, hidden from assistive technology.'
}
