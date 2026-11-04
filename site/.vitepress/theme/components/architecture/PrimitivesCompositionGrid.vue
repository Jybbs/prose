<script setup lang="ts">
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
  <div class="flex flex-col gap-8">
    <section
      v-for="band in bands"
      :key="band.key"
      class="flex flex-col gap-5 py-4"
      :data-layer="band.key"
    >
      <header class="primitives-composition-band-head flex items-center gap-9 px-4 pt-2 pb-4">
        <span class="pip size-26 shrink-0 bg-layer-accent">
          <span class="text-0-92rem leading-none select-none" aria-hidden="true">{{ band.numeral }}</span>
        </span>
        <span class="band-name">{{ band.key }}</span>
        <span class="hairline" aria-hidden="true"></span>
      </header>
      <ul class="m-0 grid list-none auto-cols-fr grid-flow-col gap-5 p-0">
        <li
          v-for="entry in band.entries"
          :key="entry.slug"
          class="m-0 rounded-sm bg-layer-accent-7-bg transition-background-and-opacity duration-180 ease-emphasized marker:content-none data-[state=active]:relative data-[state=active]:z-1 data-[state=active]:bg-layer-accent data-[state=active]:outline-2 data-[state=active]:outline-offset-1 data-[state=active]:outline-layer-accent-55-text-1 data-[state=mute]:opacity-35 data-[state=related]:bg-layer-accent-28-bg"
          :data-layer="entry.layer"
          :data-state="tileState(entry.slug)"
        >
          <a
            class="flex min-h-30 items-center justify-center px-7 py-5 text-center text-text-1 not-in-data-[state=active]:hover:text-brand-2 in-data-[state=active]:text-bg"
            :href="primitiveRoute(entry.slug)"
            @mouseenter="emit('focus', entry.slug)"
            @mouseleave="emit('blur')"
            @focus="emit('focus', entry.slug)"
            @blur="emit('blur')"
          >
            <span
              class="font-display text-xs leading-1-15 text-text-1 in-data-[state=active]:text-bg"
            >{{ entry.name }}</span>
          </a>
        </li>
      </ul>
    </section>
  </div>
</template>
