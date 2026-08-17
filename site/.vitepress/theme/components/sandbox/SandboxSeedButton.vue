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
      <path d="M4 9h5" />
      <path d="M14 9h6" />
      <circle cx="11.5" cy="9" r="2.2" />
      <path d="M4 15h9" />
      <path d="M18 15h2" />
      <circle cx="15.5" cy="15" r="2.2" />
    </svg>
  </a>
</template>
