<script setup lang="ts">
import Chip from '../base/Chip.vue'

import { data as stars } from '../../../data/stars.data'
import { REPO_URL }      from '../../../lib/shared/constants'
</script>

<template>
  <Chip variant="star-count" :href="REPO_URL" title="GitHub stars">
    <span class="star-glyph" aria-hidden="true">★</span>
    {{ stars.stars }}
  </Chip>
</template>
