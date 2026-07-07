<script setup lang="ts">
import { useRoute } from 'vitepress'
import { computed } from 'vue'

import { data as glossary } from '../../../lib/glossary/glossary.data'
import { lookup }           from '../../../lib/shared/lookup'
import { stripSuffix }      from '../../../lib/shared/strip-suffix'

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
  const current = stripSuffix(stripSuffix(route.path, '.html'), '/')
  return current !== stripSuffix(entry.href, '/')
})
</script>

<template>
  <span
    v-tooltip="{
      content            : tooltipHtml,
      html               : true,
      noAutoFocus        : true,
      popperHideTriggers : ['hover'],
      popperTriggers     : ['hover'],
      theme              : 'glossary'
    }"
    class="glossary-anchor underline-draw"
    tabindex="0"
  ><slot /></span>
</template>
