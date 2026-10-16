<script setup lang="ts">
import Chip   from '../../.vitepress/theme/components/base/Chip.vue'
import Emblem from './Emblem.vue'

import { EMBLEMS, type House } from './emblems'

const house = defineProps<House>()
</script>

<template>
  <figure class="icon-specimen">
    <dl class="icon-specimen-axes kicker">
      <div v-for="(value, axis) in house" :key="axis">
        <dt>{{ axis }}</dt>
        <dd>{{ value }}</dd>
      </div>
    </dl>
    <div class="icon-specimen-chips">
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
    <ul class="icon-specimen-tiles">
      <li v-for="[emoji, emblem] in EMBLEMS" :key="emoji">
        <span class="icon-specimen-tile"><Emblem :emblem="emblem" :house="house" /></span>
        <span class="kicker">{{ emblem.name }}</span>
      </li>
    </ul>
    <div class="icon-specimen-card panel">
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
