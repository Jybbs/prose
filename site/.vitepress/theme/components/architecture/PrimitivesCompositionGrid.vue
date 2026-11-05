<script setup lang="ts">
import { useTemplateRef } from 'vue'

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

const props = defineProps<{
  bands   : readonly Band[]
  focused : PrimitiveSlug | null
  ports   : ReadonlyMap<string, Port>
}>()

const emit = defineEmits<{
  (e: 'focus', slug: PrimitiveSlug): void
  (e: 'blur'): void
}>()

function tileState(slug: PrimitiveSlug): 'active' | 'related' | 'mute' | 'idle' {
  if (props.focused === null) return 'idle'
  if (slug === props.focused) return 'active'
  if (props.ports.has(slug)) return 'related'
  return 'mute'
}

const grid = useTemplateRef<HTMLElement>('grid')
const wash = useTemplateRef<HTMLElement>('wash')

useGlideWash(grid, wash, '[data-state="active"]', () => props.focused)
</script>

<template>
  <div ref="grid" class="relative isolate flex flex-col gap-8">
    <span ref="wash" class="primitives-composition-wash pointer-events-none absolute top-0 left-0 z-1 rounded-sm opacity-0 data-on:opacity-100 data-snap:transition-none motion-reduce:transition-none" aria-hidden="true"></span>
    <section
      v-for="band in bands"
      :key="band.key"
      class="flex flex-col gap-5 py-4"
      :data-layer="band.key"
    >
      <header class="relative z-2 flex items-center gap-9 px-4 pt-2 pb-4">
        <span class="inline-flex w-26 shrink-0 items-center justify-center">
          <span class="font-display text-1-5rem leading-none font-500 text-layer-ink italic select-none" aria-hidden="true">{{ band.numeral }}</span>
        </span>
        <span class="band-name">{{ band.key }}</span>
        <span class="hairline" aria-hidden="true"></span>
      </header>
      <ul class="m-0 grid list-none auto-cols-fr grid-flow-col gap-5 p-0">
        <li
          v-for="entry in band.entries"
          :key="entry.slug"
          class="relative m-0 rounded-sm bg-bg-soft transition-background-and-opacity duration-180 ease-emphasized marker:content-none data-[state=active]:bg-transparent data-[state=mute]:opacity-80 motion-reduce:transition-none"
          :data-layer="entry.layer"
          :data-state="tileState(entry.slug)"
        >
          <a
            class="relative z-2 flex min-h-30 items-center justify-center px-7 py-5 text-center text-text-1 focus-visible:focus-ring focus-visible:-outline-offset-1"
            :href="primitiveRoute(entry.slug)"
            @mouseenter="emit('focus', entry.slug)"
            @mouseleave="emit('blur')"
            @focus="emit('focus', entry.slug)"
            @blur="emit('blur')"
          >
            <span class="font-display text-xs leading-1-15 text-text-1">{{ entry.name }}</span>
          </a>
          <Transition name="primitives-composition-port">
            <span
              v-if="tileState(entry.slug) === 'related'"
              :key="ports.get(entry.slug)"
              class="absolute -top-7 left-6 z-3 port-tab bg-layer-ink"
              :data-dir="ports.get(entry.slug)"
              aria-hidden="true"
            >{{ PORT_WORDS[ports.get(entry.slug)!] }}</span>
          </Transition>
        </li>
      </ul>
    </section>
  </div>
</template>
