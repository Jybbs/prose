import { familyEmblem } from '../../lib/shared/emblems'

import type { Declaration } from '../catalog'
import { nibBadgeHtml }     from './nib'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'The markup a Markdown rule emits',
      render : () => <span innerHTML={nibBadgeHtml(familyEmblem('formatting'), 32, 'Formatting')} />
    }
  ],
  use      : 'Holds the measures of the nib drawings and writes a badge as inert markup, for a loader or a Markdown rule where no component mounts.'
}
