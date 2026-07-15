<script setup lang="ts">
import { useClipboard } from '@vueuse/core'

const props = defineProps<{ copiedDuring?: number, label: string, source: string }>()

const { copied, copy } = useClipboard({ copiedDuring: props.copiedDuring })
</script>

<template>
  <button
    type="button"
    class="copy"
    :class="{ copied }"
    :title="copied ? 'Copied' : label"
    :aria-label="copied ? 'Copied' : label"
    @click="copy(source)"
  >
    <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <rect x="8" y="2" width="8" height="4" rx="1" />
      <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
      <path v-if="copied" d="m9 14 2 2 4-4" />
    </svg>
  </button>
</template>
