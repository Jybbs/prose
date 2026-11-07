import { defineComponent, shallowRef, Transition } from 'vue'
import type { PropType }                           from 'vue'

import { PORT_WORDS, type Port }              from '../../../lib/architecture/ports'
import { useGlideWash }                       from '../../../lib/composables/use-glide-wash'
import type { PrimitiveLayer, PrimitiveSlug } from '../../../lib/shared/registries'
import { primitiveRoute }                     from '../../../lib/shared/routes'

interface BandEntry {
  layer : PrimitiveLayer
  name  : string
  slug  : PrimitiveSlug
}

interface Band {
  entries : readonly BandEntry[]
  key     : PrimitiveLayer
  numeral : string
}

export const PrimitivesCompositionGrid = defineComponent({
  name  : 'PrimitivesCompositionGrid',
  props : {
    bands   : { required: true, type: Array as PropType<readonly Band[]> },
    focused : { required: true, type: null as unknown as PropType<PrimitiveSlug | null> },
    ports   : { required: true, type: Map as PropType<ReadonlyMap<string, Port>> }
  },
  emits : { blur: () => true, focus: (slug: PrimitiveSlug) => true },
  setup(props, { emit }) {
    function tileState(slug: PrimitiveSlug): 'active' | 'related' | 'mute' | 'idle' {
      if (props.focused === null) return 'idle'
      if (slug === props.focused) return 'active'
      if (props.ports.has(slug)) return 'related'
      return 'mute'
    }

    const grid = shallowRef<HTMLElement | null>(null)
    const wash = shallowRef<HTMLElement | null>(null)

    useGlideWash(grid, wash, '[data-state="active"]', () => props.focused)

    return () => (
      <div class="relative isolate flex flex-col gap-8" ref={grid}>
        <span
          aria-hidden="true"
          class="primitives-composition-wash pointer-events-none absolute top-0 left-0 z-1 rounded-sm opacity-0 data-on:opacity-100 data-snap:transition-none motion-reduce:transition-none"
          ref={wash}
        />
        {props.bands.map(band => (
          <section class="flex flex-col gap-5 py-4" data-layer={band.key} key={band.key}>
            <header class="relative z-2 flex items-center gap-9 px-4 pt-2 pb-4">
              <span class="inline-flex w-26 shrink-0 items-center justify-center">
                <span
                  aria-hidden="true"
                  class="font-display text-1-5rem leading-none font-500 text-layer-ink italic select-none"
                >{band.numeral}</span>
              </span>
              <span class="band-name">{band.key}</span>
              <span aria-hidden="true" class="hairline" />
            </header>
            <ul class="m-0 grid list-none auto-cols-fr grid-flow-col gap-5 p-0">
              {band.entries.map(entry => (
                <li
                  class="relative m-0 rounded-sm bg-bg-soft transition-background-and-opacity duration-180 ease-emphasized marker:content-none data-[state=active]:bg-transparent data-[state=mute]:opacity-80 motion-reduce:transition-none"
                  data-layer={entry.layer}
                  data-state={tileState(entry.slug)}
                  key={entry.slug}
                >
                  <a
                    class="relative z-2 flex min-h-30 items-center justify-center px-7 py-5 text-center text-text-1 focus-visible:focus-ring focus-visible:-outline-offset-1"
                    href={primitiveRoute(entry.slug)}
                    onBlur={() => emit('blur')}
                    onFocus={() => emit('focus', entry.slug)}
                    onMouseenter={() => emit('focus', entry.slug)}
                    onMouseleave={() => emit('blur')}
                  >
                    <span class="font-display text-xs leading-1-15 text-text-1">{entry.name}</span>
                  </a>
                  <Transition name="primitives-composition-port">
                    {tileState(entry.slug) === 'related' && (
                      <span
                        aria-hidden="true"
                        class="absolute -top-7 left-6 z-3 port-tab bg-layer-ink"
                        data-dir={props.ports.get(entry.slug)}
                        key={props.ports.get(entry.slug)}
                      >{PORT_WORDS[props.ports.get(entry.slug)!]}</span>
                    )}
                  </Transition>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    )
  }
})
