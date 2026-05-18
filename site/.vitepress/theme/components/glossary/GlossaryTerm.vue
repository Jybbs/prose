<script setup lang="ts">
import './glossary.css'

import { vTooltip } from 'floating-vue'
import { computed } from 'vue'

import { data as glossary } from '../../../data/glossary.data'
import { lookup }           from '../../../lib/shared/registry'

const props = defineProps<{ slug: string }>()

const entry = computed(() => lookup(glossary.entries, props.slug, 'Glossary entry'))

const tooltipContent = computed(() => {
  const parts = [
    `<div class="glossary-tooltip-title">${props.slug}</div>`,
    `<div class="glossary-tooltip-divider" aria-hidden="true"></div>`,
    `<div class="glossary-tooltip-body">${entry.value.definitionHtml}</div>`
  ]
  if (entry.value.href) {
    parts.push(`<a href="${entry.value.href}" class="glossary-tooltip-link">Read more →</a>`)
  }
  return parts.join('')
})
</script>

<template>
  <span
    v-tooltip="{ content: tooltipContent, html: true, theme: 'glossary' }"
    class="glossary-anchor"
    tabindex="0"
  ><slot /></span>
</template>
