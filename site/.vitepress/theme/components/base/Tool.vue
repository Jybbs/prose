<script setup lang="ts">
import { computed } from 'vue'

import ToolIcon from './ToolIcon.vue'

import { data as tools } from '../../../lib/shared/tools.data'
import { externalAttrs } from '../../../lib/shared/links'
import { lookup }        from '../../../lib/shared/lookup'

const props = defineProps<{
  slug : string
}>()

const entry = computed(() => lookup(tools.entries, props.slug, 'Tool'))
</script>

<template>
  <a
    class="tool-mark"
    :href="entry.href"
    :title="entry.name"
    v-bind="externalAttrs(entry.href)"
  >
    <code>
      <ToolIcon class="tool-mark-icon mr-3 inline-block size-0-78em" :slug="slug" />{{ entry.name }}
    </code>
  </a>
</template>
