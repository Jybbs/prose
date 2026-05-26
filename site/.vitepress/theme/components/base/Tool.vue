<script setup lang="ts">
import { computed } from 'vue'

import { data as tools }    from '../../../data/tools.data'
import { lookup }            from '../../../lib/shared/lookup'

const props = defineProps<{
  bare ?: boolean
  slug  : string
}>()

const entry = computed(() => lookup(tools.entries, props.slug, 'Tool'))
</script>

<template>
  <a
    class="tool-mark"
    :class="{ 'tool-bare': bare }"
    :href="entry.href"
    :title="entry.name"
    target="_blank"
    rel="noopener"
  >
    <code v-if="!bare">
      <svg
        class="tool-mark-icon"
        :viewBox="entry.icon.viewBox"
        aria-hidden="true"
        v-html="entry.icon.body"
      />{{ entry.name }}
    </code>
    <svg
      v-else
      class="tool-mark-icon"
      :viewBox="entry.icon.viewBox"
      aria-hidden="true"
      v-html="entry.icon.body"
    />
  </a>
</template>
