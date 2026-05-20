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
    <svg class="tool-mark-icon" :viewBox="entry.icon.viewBox" aria-hidden="true" v-html="entry.icon.body" />
    <span v-if="!bare" class="tool-mark-name">{{ entry.name }}</span>
  </a>
</template>

<style scoped>
.tool-mark {
  display         : inline-flex;
  align-items     : center;
  gap             : 5px;
  text-decoration : none;
  color           : inherit;
  vertical-align  : baseline;
}

.tool-mark-icon {
  width      : 1em;
  height     : 1em;
  flex-shrink: 0;
}

.tool-mark-name {
  font-family : var(--vp-font-family-mono);
  font-size   : 0.92em;
  color       : var(--vp-c-text-1);
  border-bottom: 1px solid transparent;
  transition  : border-color var(--prose-transition), color var(--prose-transition);
}

.tool-mark:hover .tool-mark-name {
  color         : var(--vp-c-brand-1);
  border-color  : currentColor;
}

.tool-bare .tool-mark-icon {
  width  : 1.1em;
  height : 1.1em;
}
</style>
