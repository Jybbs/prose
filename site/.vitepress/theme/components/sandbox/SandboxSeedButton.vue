<script setup lang="ts">
import { computedAsync } from '@vueuse/core'

import { seedUrl, type SavedSession } from '../../../lib/sandbox/session'

const props = defineProps<{ seed: SavedSession }>()

// The payload deflates through `CompressionStream`, so the link resolves
// client-side and stays empty where the platform lacks the codec.
const href = computedAsync(
  async () => await seedUrl(props.seed.configToml, props.seed.source) ?? '', '')
</script>

<template>
  <a
    v-if="href"
    class="panel-seat panel-corner sandbox-seed"
    :href="href"
    title="Open this case in the sandbox"
    aria-label="Open this case in the sandbox"
  >
    <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M14 4h6v6" />
      <path d="M20 4l-8 8" />
      <path d="M18 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5" />
    </svg>
  </a>
</template>
