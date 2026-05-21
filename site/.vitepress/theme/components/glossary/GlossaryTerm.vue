<script setup lang="ts">
import { vTooltip } from 'floating-vue'

import { data as glossary } from '../../../data/glossary.data'
import { lookup }           from '../../../lib/shared/lookup'

const props = defineProps<{ slug: string }>()

const entry = lookup(glossary.entries, props.slug, 'Glossary entry')
</script>

<template>
  <span
    v-tooltip="{
      content: entry.tooltipHtml,
      html: true,
      theme: 'glossary',
      delay: { show: 80, hide: 240 },
      popperTriggers: ['hover'],
      popperHideTriggers: ['hover']
    }"
    class="glossary-anchor"
    tabindex="0"
  ><slot /></span>
</template>
