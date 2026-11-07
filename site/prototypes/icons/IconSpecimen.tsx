import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import { Chip }   from '../../.vitepress/kit'
import { Emblem } from './Emblem'

import { EMBLEMS, type House } from './emblems'

export const IconSpecimen = defineComponent({
  name  : 'IconSpecimen',
  props : {
    form     : { required: true, type: String as PropType<House['form']> },
    terminal : { required: true, type: String as PropType<House['terminal']> },
    tone     : { required: true, type: String as PropType<House['tone']> },
    weight   : { required: true, type: Number as PropType<House['weight']> }
  },
  setup(house) {
    return () => (
      <figure class="mt-24 mb-40 flex flex-col gap-20">
        <dl class="icon-specimen-axes kicker flex flex-wrap gap-x-18 gap-y-6">
          {Object.entries(house).map(([axis, value]) => (
            <div class="flex gap-6" key={axis}>
              <dt>{axis}</dt>
              <dd class="text-text-1">{value}</dd>
            </div>
          ))}
        </dl>
        <div class="icon-specimen-chips flex flex-wrap gap-8">
          {[...EMBLEMS].map(([emoji, emblem]) => (
            <Chip family={emblem.family} key={emoji} variant="family-chip">
              {{
                default: () => [
                  <Emblem emblem={emblem} house={house} />,
                  <span>{emblem.name}</span>
                ]
              }}
            </Chip>
          ))}
        </div>
        <ul class="m-0 grid list-none grid-cols-fill-96 gap-x-8 gap-y-14 p-0">
          {[...EMBLEMS].map(([emoji, emblem]) => (
            <li class="m-0 flex flex-col items-center gap-6" key={emoji}>
              <span class="icon-specimen-tile medallion size-32 border-divider text-1-05rem">
                <Emblem emblem={emblem} house={house} />
              </span>
              <span class="kicker">{emblem.name}</span>
            </li>
          ))}
        </ul>
        <div class="icon-specimen-card flex flex-wrap justify-center gap-28 panel bg-panel-wash p-28">
          {[...EMBLEMS].map(([emoji, emblem]) => <Emblem emblem={emblem} house={house} key={emoji} standalone />)}
        </div>
      </figure>
    )
  }
})
