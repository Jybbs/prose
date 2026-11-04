<script setup lang="ts">
import { useRoute } from 'vitepress'
import { computed } from 'vue'

import { useAriaHidden }    from '../../../lib/composables/use-aria-hidden'
import { data as glossary } from '../../../lib/glossary/glossary.data'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { lookup }           from '../../../lib/shared/lookup'
import { nibBadgeHtml }     from '../../../lib/shared/nib'
import { stripSuffix }      from '../../../lib/shared/strip-suffix'

const props = defineProps<{ slug: string }>()

const ariaHidden = useAriaHidden()
const entry      = lookup(glossary.entries, props.slug, 'Glossary entry')
const route      = useRoute()

const families = entry.families

// Heads the tooltip on one line with the entry's title and, at its right
// end, each family's nib, a middle dot, and the Glossary kicker.
const head = [
  `<div class="glossary-tooltip-title">${entry.slug}</div>`,
  '<div class="nib-tag">',
  ...families.map(family => {
    const emblem = familyEmblem(family)
    return `<span class="glossary-tooltip-nib" data-family="${family}">${nibBadgeHtml(emblem, 14, emblem.name)}</span>`
  }),
  families.length > 0 ? '<span class="middle-dot" aria-hidden="true">·</span>' : '',
  '<span class="kicker">Glossary</span>',
  '</div>'
].join('')

const tooltipHtml = computed(() => {
  const parts = [
    `<div class="glossary-tooltip-head">${head}</div>`,
    `<div class="glossary-tooltip-body">${entry.definitionHtml}</div>`
  ]
  if (showLink.value) {
    parts.push(`<a href="${entry.href}" class="glossary-tooltip-link"><span class="nib-stroke">Read more →</span></a>`)
  }
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
    :tabindex="ariaHidden ? -1 : 0"
  ><slot /></span>
</template>
