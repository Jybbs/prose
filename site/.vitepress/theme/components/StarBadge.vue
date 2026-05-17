<script setup lang="ts">
import { REPO_URL }       from '../../lib/constants'
import { data as stars }  from '../../data/stars.data'
</script>

<template>
  <a
    :href="REPO_URL"
    class="star-count"
    title="GitHub stars"
    target="_blank"
    rel="noopener"
  >
    <span class="star-glyph" aria-hidden="true">★</span>
    {{ stars.stars }}
  </a>
</template>
