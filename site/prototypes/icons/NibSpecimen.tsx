import { defineComponent } from 'vue'

import { Chip, nib, NibBadge } from '../../.vitepress/kit'

import { EMBLEMS } from './emblems'

export const NibSpecimen = defineComponent({
  name: 'NibSpecimen',
  setup() {
    const axes = {
      form     : 'line',
      terminal : 'broad nib',
      tone     : 'ink on a neutral tint',
      weight   : nib.NIB.stroke.nib
    }

    return () => (
      <figure class="mt-24 mb-40 flex flex-col gap-20">
        <dl class="icon-specimen-axes kicker flex flex-wrap gap-x-18 gap-y-6">
          {Object.entries(axes).map(([axis, value]) => (
            <div class="flex gap-6" key={axis}>
              <dt>{axis}</dt>
              <dd class="text-text-1">{value}</dd>
            </div>
          ))}
        </dl>
        <div class="flex flex-wrap gap-8">
          {[...EMBLEMS].map(([emoji, emblem]) => (
            <Chip class="nib-ground" family={emblem.family} key={emoji} variant="family-chip">
              {{
                default: () => [
                  <NibBadge emblem={emblem} size={15.2} />,
                  <span>{emblem.name}</span>
                ]
              }}
            </Chip>
          ))}
        </div>
        <ul class="m-0 grid list-none grid-cols-fill-96 gap-x-8 gap-y-14 p-0">
          {[...EMBLEMS].map(([emoji, emblem]) => (
            <li class="m-0 flex flex-col items-center gap-6" data-family={emblem.family} key={emoji}>
              <span class="icon-specimen-tile medallion size-32 border-divider nib-ground text-1-05rem">
                <NibBadge emblem={emblem} size={16.8} />
              </span>
              <span class="kicker">{emblem.name}</span>
            </li>
          ))}
        </ul>
        <div class="icon-specimen-card flex flex-wrap justify-center gap-28 panel bg-panel-wash p-28">
          {[...EMBLEMS].map(([emoji, emblem]) => (
            <span data-family={emblem.family} key={emoji}>
              <NibBadge emblem={emblem} label={emblem.motif.label} size={96} />
            </span>
          ))}
        </div>
      </figure>
    )
  }
})
