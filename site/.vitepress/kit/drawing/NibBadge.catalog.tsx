import { familyEmblem } from '../../lib/shared/emblems'

import type { Declaration } from '../catalog'
import { NibBadge }         from './NibBadge'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'Labeled as an image',
      render : () => <NibBadge emblem={familyEmblem('alignment')} label="Alignment" size={32} />
    },
    {
      label  : 'Decorative, at the small drawing',
      render : () => <NibBadge emblem={familyEmblem('ordering')} size={14} />
    }
  ],
  use      : 'Draws one emblem from the sprite the page carries, hidden from assistive technology unless it takes a label.'
}
