<script setup lang="ts">
import { data as stars } from '../../../data/stars.data'
import { REPO_URL }      from '../../../lib/shared/constants'
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
