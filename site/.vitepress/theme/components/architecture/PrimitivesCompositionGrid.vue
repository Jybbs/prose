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

useGlideWash(grid, wash, '.primitives-composition-tile-active', () => props.focused)
</script>

<template>
  <div ref="grid" class="primitives-composition-grid">
    <span ref="wash" class="primitives-composition-wash" aria-hidden="true"></span>
    <section
      v-for="band in bands"
      :key="band.key"
      class="primitives-composition-band"
      :data-layer="band.key"
    >
      <header class="primitives-composition-band-head band-head">
        <span class="primitives-composition-band-badge">
          <span class="primitives-composition-band-numeral" aria-hidden="true">{{ band.numeral }}</span>
        </span>
        <span class="primitives-composition-band-name band-name">{{ band.key }}</span>
        <span class="primitives-composition-band-rule band-rule" aria-hidden="true"></span>
      </header>
      <ul class="primitives-composition-band-cells">
        <li
          v-for="entry in band.entries"
          :key="entry.slug"
          class="primitives-composition-tile"
          :class="{
            'primitives-composition-tile-active' : tileState(entry.slug) === 'active',
            'primitives-composition-tile-related': tileState(entry.slug) === 'related',
            'primitives-composition-tile-mute'   : tileState(entry.slug) === 'mute'
          }"
          :data-layer="entry.layer"
        >
          <a
            class="primitives-composition-tile-link"
            :href="primitiveRoute(entry.slug)"
            @mouseenter="emit('focus', entry.slug)"
            @mouseleave="emit('blur')"
            @focus="emit('focus', entry.slug)"
            @blur="emit('blur')"
          >
            <span class="primitives-composition-tile-name">{{ entry.name }}</span>
          </a>
          <Transition name="primitives-composition-port">
            <span
              v-if="tileState(entry.slug) === 'related'"
              :key="ports.get(entry.slug)"
              class="primitives-composition-port"
              :data-dir="ports.get(entry.slug)"
              aria-hidden="true"
            >{{ PORT_WORDS[ports.get(entry.slug)!] }}</span>
          </Transition>
        </li>
      </ul>
    </section>
  </div>
</template>
