<script setup lang="ts">
import Kicker from '../base/Kicker.vue'

import { data as rules } from '../../../data/rules.data'

const slugs   = rules.list.map(r => r.slug)
const doubled = [...slugs, ...slugs]
</script>

<template>
  <div class="landing-rules" aria-label="Rules catalog">
    <Kicker class="landing-rules-kicker">— the catalog · {{ slugs.length }} rules, 1 toml file —</Kicker>
    <div class="landing-rules-track">
      <a
        v-for="(slug, idx) in doubled"
        :key="idx"
        :aria-hidden="idx >= slugs.length"
        :href="`/rules/${slug}`"
      >{{ slug }}</a>
    </div>
  </div>
</template>
