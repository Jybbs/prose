<script setup lang="ts">
import Chip     from '../../.vitepress/theme/components/base/Chip.vue'
import NibBadge from '../../.vitepress/theme/components/base/NibBadge.vue'

import { NIB }     from '../../.vitepress/lib/shared/nib'
import { EMBLEMS } from './emblems'

const axes = {
  form     : 'line',
  terminal : 'broad nib',
  tone     : 'ink on a neutral tint',
  weight   : NIB.stroke.nib
}
</script>

<template>
  <figure class="icon-specimen">
    <dl class="icon-specimen-axes kicker">
      <div v-for="(value, axis) in axes" :key="axis">
        <dt>{{ axis }}</dt>
        <dd>{{ value }}</dd>
      </div>
    </dl>
    <div class="icon-specimen-chips">
      <Chip
        v-for="[emoji, emblem] in EMBLEMS"
        :key="emoji"
        class="nib-ground"
        variant="family-chip"
        :family="emblem.family"
      >
        <NibBadge :emblem="emblem" :size="15.2" />
        <span>{{ emblem.name }}</span>
      </Chip>
    </div>
    <ul class="icon-specimen-tiles">
      <li v-for="[emoji, emblem] in EMBLEMS" :key="emoji" :data-family="emblem.family">
        <span class="icon-specimen-tile nib-ground"><NibBadge :emblem="emblem" :size="16.8" /></span>
        <span class="kicker">{{ emblem.name }}</span>
      </li>
    </ul>
    <div class="icon-specimen-card panel">
      <span v-for="[emoji, emblem] in EMBLEMS" :key="emoji" :data-family="emblem.family">
        <NibBadge :emblem="emblem" :label="emblem.motif.label" :size="96" />
      </span>
    </div>
  </figure>
</template>
