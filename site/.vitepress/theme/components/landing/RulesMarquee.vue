<script setup lang="ts">
import Kicker from '../Kicker.vue'

import { data as rules } from '../../../data/rules.data'

const slugs = rules.list.map(r => r.slug)
</script>

<template>
  <div class="landing-rules" aria-label="Rules catalog">
    <Kicker class="landing-rules-kicker">— the catalog · {{ slugs.length }} rules, 1 toml file —</Kicker>
    <div class="landing-rules-track">
      <a v-for="(slug, idx) in slugs"           :key="`a-${idx}`" :href="`/rules/${slug}`">{{ slug }}</a>
      <a v-for="(slug, idx) in slugs" aria-hidden="true" :key="`b-${idx}`" :href="`/rules/${slug}`">{{ slug }}</a>
    </div>
  </div>
</template>
