<script setup lang="ts">
import type { RenderedLine } from '../../../lib/reference/run-summary'

defineProps<{ line: RenderedLine }>()
</script>

<template>
  <div class="overflow-hidden panel bg-code-block font-mono text-md">
    <div class="flex items-center gap-7 border-b-1 border-b-divider bg-bg-soft px-14 py-9">
      <span v-for="n in 3" :key="n" class="size-11 rounded-half bg-text-3 opacity-35" aria-hidden="true" />
      <span class="ms-6 text-xs tracking-0-05em text-text-3" aria-hidden="true">prose</span>
      <slot name="bar" />
    </div>
    <div class="min-h-1-5em px-22 py-18 leading-1-5">
      <div class="flex items-baseline gap-0-65em">
        <span
          v-if="line.anchor"
          class="run-summary-anchor shrink-0 basis-1-4em text-center data-ube:text-ube"
          :data-ube="line.anchorUbe || undefined"
          aria-hidden="true"
        >{{ line.anchor }}</span>
        <span class="run-summary-msg data-[tint=apricot]:text-apricot-code-45 data-[tint=celadon]:text-celadon-code-45" :data-tint="line.countTint || undefined">{{ line.text }}</span>
      </div>
    </div>
  </div>
</template>
