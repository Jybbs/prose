<script setup lang="ts">
import { data as primitiveMeta } from '../../../data/primitives.data'

import type { PrimitiveLayer, PrimitiveSlug } from '../../../lib/shared/registries'

interface BandEntry {
  layer : PrimitiveLayer
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
  related : ReadonlySet<string>
}>()

const emit = defineEmits<{
  (e: 'focus', slug: PrimitiveSlug): void
  (e: 'blur'): void
}>()

function tileState(slug: PrimitiveSlug): 'active' | 'related' | 'mute' | 'idle' {
  if (props.focused === null) return 'idle'
  if (slug === props.focused) return 'active'
  if (props.related.has(slug)) return 'related'
  return 'mute'
}
</script>

<template>
  <div class="primitives-composition-grid">
    <section
      v-for="band in bands"
      :key="band.key"
      class="primitives-composition-band"
      :data-layer="band.key"
    >
      <header class="primitives-composition-band-head">
        <span class="primitives-composition-band-badge">
          <span class="primitives-composition-band-numeral" aria-hidden="true">{{ band.numeral }}</span>
        </span>
        <span class="primitives-composition-band-name">{{ band.key }}</span>
        <span class="primitives-composition-band-rule" aria-hidden="true"></span>
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
            :href="`/primitives/${entry.slug}`"
            @mouseenter="emit('focus', entry.slug)"
            @mouseleave="emit('blur')"
            @focus="emit('focus', entry.slug)"
            @blur="emit('blur')"
          >
            <span class="primitives-composition-tile-name">{{ primitiveMeta.bySlug[entry.slug].name }}</span>
          </a>
        </li>
      </ul>
    </section>
  </div>
</template>
