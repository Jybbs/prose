<script setup lang="ts">
import { Chip } from '../../.vitepress/kit'
import Emblem   from './Emblem.vue'

import { EMBLEMS, type House } from './emblems'

const house = defineProps<House>()
</script>

<template>
  <figure class="mt-24 mb-40 flex flex-col gap-20">
    <dl class="icon-specimen-axes kicker flex flex-wrap gap-x-18 gap-y-6">
      <div v-for="(value, axis) in house" :key="axis" class="flex gap-6">
        <dt>{{ axis }}</dt>
        <dd class="text-text-1">{{ value }}</dd>
      </div>
    </dl>
    <div class="icon-specimen-chips flex flex-wrap gap-8">
      <Chip
        v-for="[emoji, emblem] in EMBLEMS"
        :key="emoji"
        variant="family-chip"
        :family="emblem.family"
      >
        <Emblem :emblem="emblem" :house="house" />
        <span>{{ emblem.name }}</span>
      </Chip>
    </div>
    <ul class="m-0 grid list-none grid-cols-fill-96 gap-x-8 gap-y-14 p-0">
      <li v-for="[emoji, emblem] in EMBLEMS" :key="emoji" class="m-0 flex flex-col items-center gap-6">
        <span class="icon-specimen-tile medallion size-32 border-divider text-1-05rem"><Emblem :emblem="emblem" :house="house" /></span>
        <span class="kicker">{{ emblem.name }}</span>
      </li>
    </ul>
    <div class="icon-specimen-card flex flex-wrap justify-center gap-28 panel bg-panel-wash p-28">
      <Emblem
        v-for="[emoji, emblem] in EMBLEMS"
        :key="emoji"
        standalone
        :emblem="emblem"
        :house="house"
      />
    </div>
  </figure>
</template>
