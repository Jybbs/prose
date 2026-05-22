<script setup lang="ts">
import { useRoute } from 'vitepress'
import { computed } from 'vue'

import { data as glossary } from '../../../data/glossary.data'
import { lookup }           from '../../../lib/shared/lookup'

const props = defineProps<{ slug: string }>()

const entry = lookup(glossary.entries, props.slug, 'Glossary entry')
const route = useRoute()

const tooltipHtml = computed(() => {
  const parts = [
    `<div class="glossary-tooltip-title">${entry.slug}</div>`,
    `<div class="glossary-tooltip-divider" aria-hidden="true"></div>`,
    `<div class="glossary-tooltip-body">${entry.definitionHtml}</div>`
  ]
  if (showLink.value) parts.push(`<a href="${entry.href}" class="glossary-tooltip-link">Read more →</a>`)
  return parts.join('')
})

const showLink = computed(() => {
  if (!entry.href) return false
  if (entry.href.includes('#')) return true
  const current = route.path.replace(/\.html$/, '').replace(/\/$/, '')
  return current !== entry.href.replace(/\/$/, '')
})
</script>

<template>
  <span
    v-tooltip="{
      content            : tooltipHtml,
      delay              : { hide: 320, show: 80 },
      html               : true,
      popperHideTriggers : ['hover'],
      popperTriggers     : ['hover'],
      theme              : 'glossary'
    }"
    class="glossary-anchor"
    tabindex="0"
  ><slot /></span>
</template>
