<script setup lang="ts">
import Chip     from '../../.vitepress/theme/components/base/Chip.vue'
import NibBadge from '../../.vitepress/theme/components/base/NibBadge.vue'

import { NIB }     from '../../.vitepress/lib/shared/nib'
import { EMBLEMS } from './emblems'

const axes = {
  form     : 'line',
  terminal : 'broad nib',
  tone     : 'family ink on a family tint',
  weight   : NIB.stroke.nib
}
</script>

<template>
  <figure class="mt-24 mb-40 flex flex-col gap-20">
    <dl class="icon-specimen-axes kicker flex flex-wrap gap-x-18 gap-y-6">
      <div v-for="(value, axis) in axes" :key="axis" class="flex gap-6">
        <dt>{{ axis }}</dt>
        <dd class="text-text-1">{{ value }}</dd>
      </div>
    </dl>
    <div class="flex flex-wrap gap-8">
      <Chip
        v-for="[emoji, emblem] in EMBLEMS"
        :key="emoji"
        class="ground"
        variant="family-chip"
        :family="emblem.family"
      >
        <NibBadge :emblem="emblem" :size="15.2" />
        <span>{{ emblem.name }}</span>
      </Chip>
    </div>
    <ul class="m-0 grid list-none grid-cols-fill-96 gap-x-8 gap-y-14 p-0">
      <li v-for="[emoji, emblem] in EMBLEMS" :key="emoji" class="m-0 flex flex-col items-center gap-6" :data-family="emblem.family">
        <span class="icon-specimen-tile medallion size-32 border-divider ground text-1-05rem"><NibBadge :emblem="emblem" :size="16.8" /></span>
        <span class="kicker">{{ emblem.name }}</span>
      </li>
    </ul>
    <div class="icon-specimen-card flex flex-wrap justify-center gap-28 panel bg-panel-wash p-28">
      <span v-for="[emoji, emblem] in EMBLEMS" :key="emoji" :data-family="emblem.family">
        <NibBadge :emblem="emblem" :label="emblem.motif.label" :size="96" />
      </span>
    </div>
  </figure>
</template>
