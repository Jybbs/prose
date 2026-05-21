<script setup lang="ts">
import { data as releases } from '../../../data/pypi-releases.data'
</script>

<template>
  <ol class="pypi-releases">
    <li v-for="(r, i) in releases" :key="r.version" :data-current="i === 0 || undefined">
      <a class="pypi-releases-version" :href="r.url" target="_blank" rel="noopener">
        <span class="pypi-releases-num">{{ r.version }}</span>
        <span class="pypi-releases-arrow" aria-hidden="true">↗</span>
      </a>
      <span class="pypi-releases-date">{{ r.date }}</span>
    </li>
  </ol>
</template>
