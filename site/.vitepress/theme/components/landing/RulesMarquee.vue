<script setup lang="ts">
import { data as rules } from '../../../data/rules.data'

const slugs = rules.map(r => r.slug)
const track = [...slugs, ...slugs]
</script>

<template>
  <div class="landing-rules" aria-label="Rules catalog">
    <p>— the catalog · {{ slugs.length }} rules, 1 toml file —</p>
    <div class="landing-rules-track">
      <a v-for="(slug, idx) in track" :key="idx" :href="`/rules/${slug}`">{{ slug }}</a>
    </div>
  </div>
</template>
