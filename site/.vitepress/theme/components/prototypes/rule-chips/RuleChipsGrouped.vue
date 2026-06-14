<script setup lang="ts">
import { computed } from 'vue'

import { CHIPS } from './rule-chips-data'

const groups = computed(() => {
  const order: string[] = []
  const bins  = new Map<string, string[]>()

  for (const { family, slug } of CHIPS) {
    if (!bins.has(family)) {
      bins.set(family, [])
      order.push(family)
    }
    bins.get(family)!.push(slug)
  }

  return order.map((family) => ({ family, slugs: bins.get(family)! }))
})
</script>

<template>
  <div class="rc-grouped">
    <section v-for="group in groups" :key="group.family" class="rc-group" :data-family="group.family">
      <header class="rc-group-head">
        <span class="rc-group-swatch" aria-hidden="true" />
        <span class="rc-group-kicker">{{ group.family }}</span>
      </header>
      <ul class="rc-group-chips">
        <li v-for="slug in group.slugs" :key="slug" class="rc-chip">{{ slug }}</li>
      </ul>
    </section>
  </div>
</template>
