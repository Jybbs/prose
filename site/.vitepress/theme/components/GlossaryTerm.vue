<script setup lang="ts">
import { computed } from 'vue'
import { vTooltip } from 'floating-vue'

import { data as glossary } from '../../data/glossary.data'

const props = defineProps<{ slug: string }>()

const entry = computed(() => {
  const found = glossary.entries[props.slug]
  if (!found) {
    throw new Error(
      `Glossary entry "${props.slug}" not registered. ` +
      `Available slugs: ${Object.keys(glossary.entries).sort().join(', ')}`
    )
  }
  return found
})

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
