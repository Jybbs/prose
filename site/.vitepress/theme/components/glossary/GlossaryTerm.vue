<script setup lang="ts">
import { data as glossary } from '../../../data/glossary.data'
import { lookup }           from '../../../lib/shared/lookup'

const props = defineProps<{ slug: string }>()

const entry = lookup(glossary.entries, props.slug, 'Glossary entry')
</script>

<template>
  <span
    v-tooltip:glossary="{
      content            : entry.tooltipHtml,
      delay              : { hide: 240, show: 80 },
      html               : true,
      popperHideTriggers : ['hover'],
      popperTriggers     : ['hover']
    }"
    class="glossary-anchor"
    tabindex="0"
  ><slot /></span>
</template>
