import { familyEmblem }        from '../../lib/shared/emblems'
import { FAMILY_META }         from '../../lib/shared/registries'
import type { GlossaryFamily } from '../../lib/shared/registries'

import type { Declaration } from '../catalog'
import { NibBadge }         from './NibBadge'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'Each family, read from the sprite this page mounts',
      render : () => (
        <div class="flex flex-wrap items-center gap-0-85rem">
          {Object.keys(FAMILY_META).map(family => (
            <NibBadge emblem={familyEmblem(family as GlossaryFamily)} key={family} label={family} size={32} />
          ))}
        </div>
      )
    }
  ],
  use      : 'Holds the drawing of every emblem in one hidden block the layout mounts once, which each badge on the page reads.'
}
